use std::io::{self, Read};
use atty::Stream;
use std::sync::Arc;
use clap::{Arg, App};
use log::LevelFilter;
use std::io::stdout;
use fern::Dispatch;

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

use crate::prelude::*;
use crate::provider::yaml::{YamlFileProvider};

const VERSION: &str = "1.0.0";

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
        .level_for("parversion", LevelFilter::Trace)
        .chain(stdout())
        .apply()
        .expect("Could not initialize logging");
}

fn setup() {
    init_logging();
}

#[tokio::main]
async fn main() {
    setup();

    let matches = App::new("parversion")
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
        .arg(Arg::with_name("schema-file")
            .short('s')
            .long("schema-file")
            .value_name("SCHEMA_FILE")
            .help("Provide file as schema for translation"))
        .arg(Arg::with_name("schema-url")
            .short('S')
            .long("schema-url")
            .value_name("SCHEMA_URL")
            .help("Provide url as schema for translation"))
        .arg(Arg::with_name("version")
            .short('v')
            .long("version")
            .help("Display program version"))
        .get_matches();

    if matches.is_present("version") {
        println!("parversion {}", VERSION);
        return;
    }

    let document_format = document_format::DocumentFormat::default();

    let provider = Arc::new(YamlFileProvider::new(String::from("provider.yaml")));

    log::info!("Using yaml file provider");

    let options = Options {
        ..Options::default()
    };
    log::debug!("options: {:?}", options);

    let maybe_json_schema: Option<String> = {
        if let Some(path) = matches.value_of("schema-file") {
            let text = get_file_as_text(path).expect("Could not get schema file");
            Some(text)
        } else if let Some(url) = matches.value_of("schema-url") {
            let text = fetch_url_as_text(url).await.expect("Could not get schema from URL");
            Some(text)
        } else {
            None
        }
    };

    let document = {
        if let Ok(stdin) = load_stdin() {
            log::info!("Received data from stdin");

            if let Some(json_schema) = maybe_json_schema {
                match translation::translate_text_to_document(
                    provider.clone(),
                    stdin,
                    &Some(options),
                    &json_schema,
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to translate text from stdin: {:?}", err);
                        std::process::exit(1);
                    }
                }
            } else {
                match normalization::normalize_text_to_document(
                    provider.clone(),
                    stdin,
                    &Some(options),
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to normalize text from stdin: {:?}", err);
                        std::process::exit(1);
                    }
                }
            }
        } else if let Some(path) = matches.value_of("file") {
            log::info!("Received a file name");

            if let Some(json_schema) = maybe_json_schema {
                match translation::translate_file_to_document(
                    provider.clone(),
                    path,
                    &Some(options),
                    &json_schema,
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to translate text from stdin: {:?}", err);
                        std::process::exit(1);
                    }
                }
            } else {
                match normalization::normalize_file_to_document(
                    provider.clone(),
                    path,
                    &Some(options),
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to normalize URL: {:?}", err);
                        std::process::exit(1);
                    }
                }
            }
        } else if let Some(url) = matches.value_of("url") {
            log::info!("Received a URL");

            if let Some(json_schema) = maybe_json_schema {
                match translation::translate_url_to_document(
                    provider.clone(),
                    url,
                    &Some(options),
                    &json_schema,
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to translate text from stdin: {:?}", err);
                        std::process::exit(1);
                    }
                }
            } else {
                match normalization::normalize_url_to_document(
                    provider.clone(),
                    url,
                    &Some(options),
                ).await {
                    Ok(document) => document,
                    Err(err) => {
                        eprintln!("Failed to normalize URL: {:?}", err);
                        std::process::exit(1);
                    }
                }
            }
        } else {
            eprintln!("No valid input provided. Please provide either stdin, a file or URL.");
            std::process::exit(1);
        }
    };

    log::info!("Successfully processed document");

    println!("{}", document.to_string(&Some(document_format)));

    std::process::exit(0);
}
