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

mod basis_network;
mod basis_node;
mod basis_graph;
#[cfg(feature = "caching")]
mod cache;
mod config;
mod data_node;
mod document;
mod document_format;
mod document_node;
mod environment;
mod graph_node;
mod hash;
mod id;
mod lineage;
mod macros;
mod normalization;
mod organization;
mod profile;
mod provider;
mod transformation;
mod translation;
mod types;
mod prelude;
#[allow(dead_code)]
mod utility;
mod json_node;
mod context;
mod context_group;
mod llm;
mod meta_context;
mod schema;
mod network_analysis;
mod node_analysis;
mod schema_node;
mod schema_context;
mod path;
mod reduction;
mod mutations;
mod function;

use crate::prelude::*;
use crate::document::{Document, DocumentType};
use crate::provider::{Provider, VoidProvider};
#[cfg(feature = "yaml-provider")]
use crate::provider::yaml::{YamlFileProvider};
#[cfg(feature = "sqlite-provider")]
use crate::provider::sqlite::{SqliteProvider};
use crate::config::{CONFIG};
use crate::mutations::Mutations;

const VERSION: &str = "0.0.0";
const PROGRAM_NAME: &str = "parversion";

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
        .arg(Arg::with_name("file")
             .short('f')
             .long("file")
             .value_name("FILE")
             .help("Provide file as document for processing"))
        .arg(Arg::with_name("url")
            .short('u')
            .long("url")
            .value_name("URL")
            .help("Provide url as document for processing"))
        .arg(Arg::with_name("inline")
            .short('i')
            .long("inline")
            .value_name("INLINE")
            .help("Provide document directly in parameter"))
        .arg(Arg::with_name("schema-file")
            .long("schema-file")
            .value_name("SCHEMA_FILE")
            .help("Provide file as schema for translation"))
        .arg(Arg::with_name("schema-url")
            .long("schema-url")
            .value_name("SCHEMA_URL")
            .help("Provide url as schema for translation"))
        .arg(Arg::with_name("schema-inline")
            .long("schema-inline")
            .value_name("SCHEMA_INLINE")
            .help("Provide schema directly in parameter value for translation"))
        .arg(Arg::with_name("version")
            .short('v')
            .long("version")
            .help("Display program version"))
        .arg(Arg::with_name("document-type")
            .short('d')
            .long("document-type")
            .value_name("DOCUMENT_TYPE")
            .help("The document type : html, xml, js"))
        .get_matches()
}

async fn get_schema(matches: &clap::ArgMatches) -> Option<String> {
    if let Some(schema) = matches.value_of("schema-inline") {
        Some(schema.to_string())
    } else if let Some(path) = matches.value_of("schema-file") {
        let text = get_file_as_text(path).expect("Could not get schema file");
        Some(text)
    } else if let Some(url) = matches.value_of("schema-url") {
        let text = fetch_url_as_text(url).await.expect("Could not get schema from URL");
        Some(text)
    } else {
        None
    }
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

async fn determine_program<P: Provider + ?Sized>(
    provider: Arc<P>,
    options: Options,
    matches: &clap::ArgMatches,
    document_type: DocumentType
) -> Result<Mutations, Errors> {
    if let Ok(stdin) = load_stdin() {
        log::info!("Received data from stdin");

        reduction::reduce_text_to_mutations(
            provider.clone(),
            stdin,
            &Some(options),
            document_type.clone(),
        ).await
    } else if let Some(path) = matches.value_of("file") {
        log::info!("Received a file name");

        reduction::reduce_file_to_mutations(
            provider.clone(),
            path,
            &Some(options),
            document_type.clone(),
        ).await
    } else if let Some(url) = matches.value_of("url") {
        log::info!("Received a URL");

        reduction::reduce_url_to_mutations(
            provider.clone(),
            url,
            &Some(options),
            document_type.clone(),
        ).await
    } else if let Some(inline_document) = matches.value_of("inline") {
        log::info!("Received an inline program");

        reduction::reduce_text_to_mutations(
            provider.clone(),
            inline_document.to_string(),
            &Some(options),
            document_type.clone(),
        ).await
    } else {
        Err(Errors::DocumentNotProvided)
    }
}

async fn determine_document<P: Provider + ?Sized>(
    maybe_json_schema: Option<String>,
    provider: Arc<P>,
    options: Options,
    matches: &clap::ArgMatches
) -> Result<Document, Errors> {
    if let Ok(stdin) = load_stdin() {
        log::info!("Received data from stdin");

        if let Some(json_schema) = maybe_json_schema {
            translation::translate_text_to_document(
                provider.clone(),
                stdin,
                &Some(options),
                &json_schema,
            ).await
        } else {
            normalization::normalize_text_to_document(
                provider.clone(),
                stdin,
                &Some(options),
            ).await
        }
    } else if let Some(path) = matches.value_of("file") {
        log::info!("Received a file name");

        if let Some(json_schema) = maybe_json_schema {
            translation::translate_file_to_document(
                provider.clone(),
                path,
                &Some(options),
                &json_schema,
            ).await
        } else {
            normalization::normalize_file_to_document(
                provider.clone(),
                path,
                &Some(options),
            ).await
        }
    } else if let Some(url) = matches.value_of("url") {
        log::info!("Received a URL");

        if let Some(json_schema) = maybe_json_schema {
            translation::translate_url_to_document(
                provider.clone(),
                url,
                &Some(options),
                &json_schema,
            ).await
        } else {
            normalization::normalize_url_to_document(
                provider.clone(),
                url,
                &Some(options),
            ).await
        }
    } else if let Some(inline_document) = matches.value_of("inline") {
        log::info!("Received an inline document");

        if let Some(json_schema) = maybe_json_schema {
            translation::translate_text_to_document(
                provider.clone(),
                inline_document.to_string(),
                &Some(options),
                &json_schema,
            ).await
        } else {
            normalization::normalize_text_to_document(
                provider.clone(),
                inline_document.to_string(),
                &Some(options),
            ).await
        }
    } else {
        Err(Errors::DocumentNotProvided)
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

async fn run() -> Result<(), Errors> {
    let start = Instant::now();

    setup();

    let matches = parse_arguments();

    if matches.is_present("version") {
        println!("{} {}", PROGRAM_NAME, VERSION);
        return Ok(());
    }

    let document_type = get_document_type(&matches)?;

    let provider = init_provider().await?;

    let options = Options {
        ..Options::default()
    };
    log::debug!("options: {:?}", options);
    




    if document_type == DocumentType::JavaScript {


        let mutations = determine_program(
            provider,
            options,
            &matches,
            document_type,
        ).await?;

        log::debug!("Successfully processed program");

        println!("{}", mutations.to_string());


    } else {

    
        ///////////////

        let document_format = document_format::DocumentFormat::default();
        let maybe_json_schema: Option<String> = get_schema(&matches).await;

        let document = determine_document(
            maybe_json_schema,
            provider,
            options,
            &matches
        ).await?;

        log::info!("Successfully processed document");

        println!("{}", document.to_string(&Some(document_format)));

        //////////////



    }


    let elapsed = start.elapsed();
    log::info!("Elapsed: {:.2?}", elapsed);

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        println!("Error occurred: {:?}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
