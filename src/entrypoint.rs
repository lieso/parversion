use atty::Stream;
use clap::{Arg, ArgAction, ArgMatches, Command};
use fern::Dispatch;
use log::LevelFilter;
use std::env;
use std::io::stdout;
use std::io::{self, Read};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use std::fs;

use crate::config::CONFIG;
use crate::document::DocumentType;
use crate::document_format;
use crate::normalization;
use crate::package::Package;
use crate::prelude::*;
#[cfg(feature = "sqlite-provider")]
use crate::provider::sqlite::SqliteProvider;
#[cfg(feature = "yaml-provider")]
use crate::provider::yaml::YamlFileProvider;
use crate::provider::{Provider, VoidProvider};
use crate::translation;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROGRAM_NAME: &str = "parversion";

pub async fn run() -> Result<(), Errors> {
    let start = Instant::now();

    let _ = ensure_prerequisites()?;

    setup();

    let matches = parse_arguments();

    if matches.get_flag("version") {
        println!("{} {}", PROGRAM_NAME, VERSION);
        return Ok(());
    }

    let execution_context = init_execution_context();
    let provider = init_provider().await?;
    let options = get_options(&matches)?;
    let documents: Vec<(String, Metadata)> = get_documents(&matches).await?;
    let translation: Option<String> = get_translation(&matches).await?;
    let document_format = document_format::DocumentFormat::default();

    let package = determine_document(
        provider,
        document,
        translation,
        options,
        metadata,
        &document_format,
        execution_context.clone(),
    ).await?;

    log::info!("Successfully processed document");

    println!("{}", package.to_string());

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
    log::info!("Initializing logging...");

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{date} [{level}] {file}:{line} - {message}",
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                level = record.level(),
                file = record.file().unwrap_or("unknown"),
                line = record.line().unwrap_or(0),
                message = message
            ))
        })
        .level(LevelFilter::Off)
        .level_for(PROGRAM_NAME, LevelFilter::Trace)
        .chain(stdout())
        .apply()
        .expect("Could not initialize logging");
}

fn setup() {
    init_logging();

    let config = read_lock!(CONFIG);

    log::debug!("===========CONFIGURATION===========");
    log::debug!("{:?}", config);
}

fn handle_error(err: Errors) {
    match err {
        Errors::YamlParseError(msg)
        | Errors::FetchUrlError(msg)
        | Errors::DeficientMetaContextError(msg) => {
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
                .short('d')
                .long("documents")
                .value_name("DOCUMENTS")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Provide documents for processing"),
        )
        .arg(
            Arg::new("document-type")
                .short('t')
                .long("document-type")
                .value_name("DOCUMENT_TYPE")
                .help("The document type : html, xml, js"),
        )
        .arg(
            Arg::new("origin")
                .short('o')
                .long("origin")
                .value_name("ORIGIN")
                .required(true)
                .help("Specify the origin of the document"),
        )
        .arg(
            Arg::new("translation")
                .short('a')
                .long("translation-target")
                .value_name("TRANSLATION_TARGET")
                .help("Optional. Provide document as target output schema"),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("Display program version"),
        )
        .arg(
            Arg::new("regenerate")
                .short('r')
                .long("regenerate")
                .help("Regenerate inferences"),
        )
        .get_matches()
}

fn get_options(matches: &clap::ArgMatches) -> Result<Options, Errors> {
    Ok(Options {
        regenerate: matches.get_flag("regenerate"),
        ..Options::default()
    })
}

async fn get_translation(matches: &clap::ArgMatches) -> Result<Option<String>, Errors> {
    if let Some(translation) = matches.get_one::<String>("translation") {
        return if is_valid_url(translation) {
            let text = fetch_url_as_text(translation).await?;
            Ok(Some(text))
        } else if is_valid_unix_path(translation) {
            let text = get_file_as_text(translation)?;
            Ok(Some(text))
        } else {
            Ok(Some(translation.to_string()))
        }
    }

    Ok(None)
}

async fn get_documents(matches: &ArgMatches) -> Result<Vec<(String, Metadata)>, Errors> {
    let mut documents: Vec<(String, Metadata)> = Vec::new();

    let fallback_type: Option<DocumentType> = matches
        .get_one::<String>("document-type")
        .map(|s| parse_document_type(s))
        .transpose()?;

    let fallback_origin: String = matches
        .get_one::<String>("origin")
        .cloned()
        .expect("origin is required by clap");

    if let Some(values) = matches.get_many::<String>("documents") {
        for raw_document in values {
            let (parsed_document, partial) = parse_document(raw_document).await?;

            let dt = partial
                .document_type
                .or_else(|| fallback_type.clone())
                .ok_or(Errors::DocumentTypeNotProvided)?;

            let origin = partial.origin.unwrap_or_else(|| fallback_origin.clone());

            let md = Metadata {
                document_type: Some(dt),
                origin,
            };

            documents.push((parsed_document, md));
        }
    } else if let Ok(stdin) = load_stdin() {
        let dt = fallback_type.ok_or(Errors::DocumentTypeNotProvided)?;
        let metadata = Metadata {
            document_type: Some(dt),
            origin: fallback_origin,
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
                partial.document_type = Some(parse_document_type(&value)?);
            }
            "origin" => {
                partial.origin = Some(value.to_string());
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

async fn determine_document<P: Provider + ?Sized>(
    provider: Arc<P>,
    document: String,
    translation: Option<String>,
    options: Options,
    metadata: Metadata,
    document_format: &document_format::DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::debug!("options: {:?}", options);
    log::debug!("metadata: {:?}", metadata);

    if let Some(translation) = translation {
        let translated_document = translation::translate_text_to_document(
            provider.clone(),
            document,
            translation,
            &options,
            &metadata,
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
