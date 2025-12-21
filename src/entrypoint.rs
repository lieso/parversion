use std::io::{self, Read};
use atty::Stream;
use std::sync::Arc;
use clap::{Arg, App};
use log::LevelFilter;
use std::io::stdout;
use fern::Dispatch;
use cfg_if::cfg_if;
use dirs;
use std::path::PathBuf;
use std::fs;
use std::time::Instant;

use crate::prelude::*;
use crate::document::{DocumentType};
use crate::provider::{Provider, VoidProvider};
#[cfg(feature = "yaml-provider")]
use crate::provider::yaml::{YamlFileProvider};
#[cfg(feature = "sqlite-provider")]
use crate::provider::sqlite::{SqliteProvider};
use crate::config::{CONFIG};
use crate::package::Package;
use crate::normalization;
use crate::translation;
use crate::document_format;

const VERSION: &str = "0.0.0";
const PROGRAM_NAME: &str = "parversion";

pub async fn run() -> Result<(), Errors> {
    let start = Instant::now();

    setup();

    let matches = parse_arguments();

    if matches.is_present("version") {
        println!("{} {}", PROGRAM_NAME, VERSION);
        return Ok(());
    }

    let provider = init_provider().await?;

    let options = Options {
        ..Options::default()
    };
    log::debug!("options: {:?}", options);

    let metadata = Metadata {
        document_type: Some(get_document_type(&matches)?)
    };
    log::debug!("metadata: {:?}", metadata);

    let schema: Option<String> = get_schema(&matches).await?;

    let document: String = get_document(&matches).await?;
    
    let package = determine_document(
        provider,
        schema,
        document,
        options,
        metadata,
    ).await?;

    let document_format = document_format::DocumentFormat::default();

    log::info!("Successfully processed document/program");

    println!("{}", package.to_string(&Some(document_format)));

    let elapsed = start.elapsed();
    log::info!("Elapsed: {:.2?}", elapsed);

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
        | Errors::JsonSchemaParseError(msg)
        | Errors::DeficientMetaContextError(msg) => {
            eprintln!("Error: {}", msg);
        }
        _ => {
            eprintln!("Error: {:?}", err);
        }
    }
}

fn parse_arguments() -> clap::ArgMatches {
    App::new(PROGRAM_NAME)
        .version(VERSION)
        .arg(Arg::with_name("document")
             .short('d')
             .long("document")
             .value_name("DOCUMENT")
             .help("Provide document for processing"))
        .arg(Arg::with_name("schema")
            .long("schema")
            .value_name("SCHEMA")
            .help("Provide schema for translation"))
        .arg(Arg::with_name("version")
            .short('v')
            .long("version")
            .help("Display program version"))
        .arg(Arg::with_name("document-type")
            .short('t')
            .long("document-type")
            .value_name("DOCUMENT_TYPE")
            .help("The document type : html, xml, js"))
        .get_matches()
}

async fn get_schema(matches: &clap::ArgMatches) -> Result<Option<String>, Errors> {
    if let Some(schema) = matches.value_of("schema") {
        return if is_valid_url(schema) {
            let text = fetch_url_as_text(schema).await?;
            Ok(Some(text))
        } else if is_valid_unix_path(schema) {
            let text = get_file_as_text(schema)?;
            Ok(Some(text))
        } else if is_valid_json(schema) {
            Ok(Some(schema.to_string()))
        } else {
            Err(Errors::SchemaNotValid)
        };
    }

    Ok(None)
}

async fn get_document(matches: &clap::ArgMatches) -> Result<String, Errors> {
    if let Ok(stdin) = load_stdin() {
        log::info!("Received data from stdin");
        return Ok(stdin);
    }

    if let Some(document) = matches.value_of("document") {
        return if is_valid_url(document) {
            let text = fetch_url_as_text(document).await?;
            Ok(text)
        } else if is_valid_unix_path(document) {
            let text = get_file_as_text(document)?;
            Ok(text)
        } else {
            log::info!("Received inline document");
            Ok(document.to_string())
        }
    }

    Err(Errors::DocumentNotProvided)
}

fn get_document_type(matches: &clap::ArgMatches) -> Result<DocumentType, Errors> {
    if let Some(document_type_input) = matches.value_of("document-type") {
        match document_type_input {
            "js" => Ok(DocumentType::JavaScript),
            "html" => Ok(DocumentType::Html),
            "xml" => Ok(DocumentType::Xml),
            _ => Err(Errors::UnexpectedDocumentType),
        }
    } else {
        Err(Errors::DocumentTypeNotProvided)
    }
}

async fn determine_document<P: Provider + ?Sized>(
    provider: Arc<P>,
    schema: Option<String>,
    document: String,
    options: Options,
    metadata: Metadata,
) -> Result<Package, Errors> {
    if let Some(schema) = schema {
        translation::translate_text_to_package(
            provider.clone(),
            document,
            &options,
            &metadata,
            &schema,
        ).await
    } else {
        normalization::normalize_text_to_package(
            provider.clone(),
            document,
            &options,
            &metadata,
        ).await
    }
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

            Ok(Arc::new(SqliteProvider::new(provider_path.to_string_lossy().into_owned())))
        
        } else {
            log::warn!("Using VoidProvider, document will be completely reprocessed each time");
            Ok(Arc::new(VoidProvider))
        }
   }
}
