use atty::Stream;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::LevelFilter;
use std::env;
use std::io::stdout;
use std::io::{self, Read};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use std::fs;
use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::CONFIG;
use crate::document::{DocumentType, DocumentRole};
use crate::document_format;
use crate::normalization;
use crate::package::Package;
use crate::prelude::*;
#[cfg(feature = "sqlite-provider")]
use crate::provider::sqlite::SqliteProvider;
#[cfg(feature = "yaml-provider")]
use crate::provider::yaml::YamlFileProvider;
use crate::provider::VoidProvider;
use crate::provider::{Provider};
use crate::translation;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROGRAM_NAME: &str = "parversion";

pub async fn run() -> Result<(), Errors> {
    let start = Instant::now();

    let _ = ensure_prerequisites()?;

    setup();

    let matches = parse_arguments();

    let execution_context = init_execution_context();
    let provider = init_provider().await?;
    let options = get_options(&matches)?;
    let documents: Vec<(String, Metadata)> = get_documents(&matches).await?;
    let translation: Option<(String, Metadata)> = get_translation(&matches).await?;
    let document_format = document_format::DocumentFormat::default();

    let package = determine_documents(
        provider,
        documents,
        translation,
        options,
        &document_format,
        execution_context.clone(),
    ).await?;

    log::info!("Successfully processed document");

    if matches.get_flag("output-metadata") {
        println!("{}", serde_json::to_string(&package.document.metadata).expect("Failed to serialize document metadata"));
    } else {
        println!("{}", package.to_string());
    }

    let elapsed = start.elapsed();
    log::info!("Elapsed: {:.2?}", elapsed);

    Ok(())
}

fn ensure_prerequisites() -> Result<(), Errors> {
    let env_vars = &["OPENAI_API_KEY", "OPENROUTER_API_KEY"];

    for &var in env_vars {
        if env::var(var).is_err() {
            return Err(Errors::InsufficientPrerequisites(format!(
                "{} environment variable must be set",
                var
            )));
        }
    }

    Ok(())
}

fn load_stdin() -> io::Result<String> {
    log::trace!("In load_stdin");

    if atty::is(Stream::Stdin) {
        return Err(io::Error::new(io::ErrorKind::Other, "stdin not redirected"));
    }
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn init_logging() {
    let filter = EnvFilter::new(format!("off,{}=trace", PROGRAM_NAME));

    fmt()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .init();
}

fn setup() {
    #[cfg(debug_assertions)]
    init_logging();

    let config = read_lock!(CONFIG);

    log::debug!("===========CONFIGURATION===========");
    log::debug!("{:?}", config);
}

fn handle_error(err: Errors) {
    match err {
        Errors::YamlParseError(msg)
        | Errors::FetchUrlError(msg)
        | Errors::DeficientNormalizationContextError(msg) => {
            eprintln!("Error: {}", msg);
        }
        _ => {
            eprintln!("Error: {:?}", err);
        }
    }
}

fn parse_arguments() -> clap::ArgMatches {
    Command::new(PROGRAM_NAME)
        .version(VERSION)
        .arg(
            Arg::new("documents")
                .short('i')
                .long("documents")
                .value_name("DOCUMENTS")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Provide documents for processing"),
        )
        .arg(
            Arg::new("document-format")
                .short('f')
                .long("document-format")
                .value_name("DOCUMENT_FORMAT")
                .help("The document type : html, xml, js"),
        )
        .arg(
            Arg::new("origin")
                .short('s')
                .long("origin")
                .value_name("ORIGIN")
                .required(true)
                .help("Specify the origin of the document"),
        )
        .arg(
            Arg::new("role")
                .short('k')
                .long("role")
                .value_name("ROLE")
                .help("Document purpose: instance, schema"),
        )
        .arg(
            Arg::new("translation")
                .short('m')
                .long("translation-spec")
                .value_name("TRANSLATION_SPEC")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Optional. Provide document as target output schema"),
        )
        .arg(
            Arg::new("regenerate")
                .short('g')
                .long("regenerate")
                .action(ArgAction::SetTrue)
                .help("Regenerate inferences"),
        )
        .arg(
            Arg::new("output-metadata")
                .short('z')
                .long("output-metadata")
                .action(ArgAction::SetTrue)
                .help("Output only document metadata"),
        )
        .get_matches()
}

fn get_options(matches: &clap::ArgMatches) -> Result<Options, Errors> {
    Ok(Options {
        regenerate: matches.get_flag("regenerate"),
        ..Options::default()
    })
}

async fn get_translation(matches: &ArgMatches) -> Result<Option<(String, Metadata)>, Errors> {
    let mut documents: Vec<(String, Metadata)> = Vec::new();

    let fallback_type: Option<DocumentType> = matches
        .get_one::<String>("document-format")
        .map(|s| parse_document_type(s))
        .transpose()?;

    let fallback_origin: String = matches
        .get_one::<String>("origin")
        .cloned()
        .expect("origin is required by clap");

    let fallback_role: DocumentRole = matches
        .get_one::<String>("role")
        .map(|s| DocumentRole::from_str(s))
        .transpose()
        .map_err(|e| Errors::InvalidRole(e.to_string()))?
        .unwrap_or(DocumentRole::Instance);

    if let Some(values) = matches.get_many::<String>("translation") {
        if values.len() > 1 {
            return Err(Errors::TooManyTranslationDocuments);
        }
        
        for raw_document in values {
            let (parsed_document, partial) = parse_document(raw_document).await?;

            let dt = partial
                .document_type
                .or_else(|| fallback_type.clone())
                .ok_or(Errors::DocumentTypeNotProvided)?;

            let origin = partial.origin.unwrap_or_else(|| fallback_origin.clone());

            let role = partial.role.unwrap_or_else(|| fallback_role.clone());

            let md = Metadata {
                document_type: Some(dt),
                origin,
                role,
            };

            return Ok(Some((parsed_document, md)));
        }
    }

    Ok(None)
}

async fn get_documents(matches: &ArgMatches) -> Result<Vec<(String, Metadata)>, Errors> {
    let mut documents: Vec<(String, Metadata)> = Vec::new();

    let fallback_type: Option<DocumentType> = matches
        .get_one::<String>("document-format")
        .map(|s| parse_document_type(s))
        .transpose()?;

    let fallback_origin: String = matches
        .get_one::<String>("origin")
        .cloned()
        .expect("An origin is expected");

    let fallback_role: DocumentRole = matches
        .get_one::<String>("role")
        .map(|s| DocumentRole::from_str(s))
        .transpose()
        .map_err(|e| Errors::InvalidRole(e.to_string()))?
        .unwrap_or(DocumentRole::Instance);

    if let Some(values) = matches.get_many::<String>("documents") {
        for raw_document in values {
            let (parsed_document, partial) = parse_document(raw_document).await?;

            let dt = partial
                .document_type
                .or_else(|| fallback_type.clone())
                .ok_or(Errors::DocumentTypeNotProvided)?;

            let origin = partial.origin.unwrap_or_else(|| fallback_origin.clone());

            let role = partial.role.unwrap_or_else(|| fallback_role.clone());

            let md = Metadata {
                document_type: Some(dt),
                origin,
                role,
            };

            documents.push((parsed_document, md));
        }
    } else if let Ok(stdin) = load_stdin() {
        let dt = fallback_type.ok_or(Errors::DocumentTypeNotProvided)?;
        let metadata = Metadata {
            document_type: Some(dt),
            origin: fallback_origin,
            role: fallback_role,
        };
        return Ok(vec![(stdin, metadata)]);
    } else {
        return Err(Errors::DocumentNotProvided);
    }

    Ok(documents)
}

#[derive(Default)]
struct MetadataPartial {
    document_type: Option<DocumentType>,
    origin: Option<String>,
    role: Option<DocumentRole>,
}

async fn parse_document(raw_document: &str) -> Result<(String, MetadataPartial), Errors> {
    let mut text: Option<String> = None;
    let mut partial = MetadataPartial::default();

    for pair in raw_document.split(',') {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| Errors::UnexpectedParameter(pair.to_string()))?;
        let key: &str = key.trim();
        let value: &str = value.trim();

        match key {
            "uri" => {
                log::debug!("uri={}", value);
                if value == "-" {
                    text = Some(load_stdin().map_err(|_| Errors::DocumentNotProvided)?);
                } else if is_valid_url(value) {
                    text = Some(fetch_url_as_text(value).await?);
                } else if is_valid_unix_path(value) {
                    text = Some(get_file_as_text(value)?);
                } else {
                    text = Some(value.to_string());
                }
            }
            "type" => {
                log::debug!("type={}", value);
                partial.document_type = Some(parse_document_type(&value)?);
            }
            "origin" => {
                log::debug!("origin={}", value);
                partial.origin = Some(value.to_string());
            }
            "role" => {
                log::debug!("role={}", value);
                partial.role = Some(DocumentRole::from_str(value).map_err(|e| Errors::InvalidRole(e.to_string()))?);
            }
            _ => return Err(Errors::UnexpectedParameter(key.to_string())),
        }
    }

    let text = text.ok_or(Errors::DocumentNotProvided)?;
    Ok((text, partial))
}

fn parse_document_type(s: &str) -> Result<DocumentType, Errors> {
    match s {
        "js" => Ok(DocumentType::JavaScript),
        "json" => Ok(DocumentType::Json),
        "html" => Ok(DocumentType::Html),
        "xml" => Ok(DocumentType::Xml),
        _ => Err(Errors::UnexpectedDocumentType),
    }
}

async fn determine_documents<P: Provider + ?Sized>(
    provider: Arc<P>,
    documents: Vec<(String, Metadata)>,
    translation: Option<(String, Metadata)>,
    options: Options,
    document_format: &document_format::DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::debug!("options: {:?}", options);

    let Some((document, metadata)) = documents.into_iter().next() else {
        unimplemented!();
    };

    log::debug!("metadata: {:?}", metadata);

    if let Some((translation, translation_metadata)) = translation {
        let translated_document = translation::translate_text_to_document(
            provider.clone(),
            (document, &metadata),
            (translation, &translation_metadata),
            &options,
            document_format,
            execution_context.clone(),
        )
        .await?;

        Ok(Package {
            document: translated_document,
            mutations: Vec::new(),
        })
    } else {
        let normalized_document = normalization::normalize_text_to_document(
            provider.clone(),
            document,
            &options,
            &metadata,
            document_format,
            execution_context.clone(),
        )
        .await?;

        Ok(Package {
            document: normalized_document,
            mutations: Vec::new(),
        })
    }
}

fn init_execution_context() -> Arc<ExecutionContext> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let execution_context = ExecutionContext::with_progress(tx);

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("\x1b[38;2;255;0;255m{:?}\x1b[0m", event); // fuchsia
        }
    });

    execution_context
}

async fn init_provider() -> Result<Arc<impl Provider>, Errors> {
    log::info!("Initializing data provider...");

    cfg_if::cfg_if! {
         if #[cfg(feature = "yaml-provider")] {
             log::info!("Using yaml file provider");

             let data_dir = dirs::data_dir()
                 .ok_or_else(|| Errors::ProviderError("Could not find data directory".into()))?;

             let provider_path = data_dir.join(PROGRAM_NAME).join("provider.yaml");

             if let Some(parent_dir) = provider_path.parent() {
                 fs::create_dir_all(parent_dir).expect("Unable to create directory");
             }

             log::debug!("provider_path: {}", provider_path.display());

             Ok(Arc::new(YamlFileProvider::new(provider_path.to_string_lossy().into_owned())))

         } else if #[cfg(feature = "sqlite-provider")] {
             log::info!("Using sqlite provider");

             let data_dir = dirs::data_dir()
                 .ok_or_else(|| Errors::ProviderError("Could not find data directory".into()))?;

             let provider_path = data_dir.join(PROGRAM_NAME).join("provider.sqlite");

             if let Some(parent_dir) = provider_path.parent() {
                 fs::create_dir_all(parent_dir).expect("Unable to create directory");
             }

             log::debug!("provider_path: {}", provider_path.display());

             let provider = SqliteProvider::new(&provider_path.to_string_lossy())?;
             Ok(Arc::new(provider))

         } else {
             log::warn!("Using VoidProvider, document will be completely reprocessed each time");
             Ok(Arc::new(VoidProvider))
         }
    }
}
