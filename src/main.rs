use std::io::{self, Read};
use std::collections::{HashMap, HashSet};
use lazy_static::lazy_static;
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use std::fs::File;
use std::io::stdout;
use fern::Dispatch;
use async_trait::async_trait;
use quick_js::{Context, JsValue};

mod analysis;
mod basis_network;
mod basis_node;
mod basis_graph;
mod config;
mod context;
mod data_node;
mod document;
mod document_format;
mod document_profile;
mod environment;
mod hash;
mod id;
mod lineage;
mod macros;
mod model;
mod normalization;
mod organization;
mod provider;
mod transformation;
mod translation;
mod types;
mod prelude;
mod utility;
mod json_node;

use crate::prelude::*;
use crate::config::{CONFIG};
use crate::document_profile::DocumentProfile;
use crate::provider::{YamlFileProvider};
use crate::transformation::{
    Runtime,
    DocumentTransformation,
    XMLElementTransformation,
};

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

    let path = format!("{}/{}", read_lock!(CONFIG).dev.debug_dir, "debug.log");
    let log_file = File::create(path).expect("Could not create log file");

    Dispatch::new()
        .level(LevelFilter::Off)
        .level_for("parversion", LevelFilter::Trace)
        .chain(stdout())
        .chain(log_file)
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
        .get_matches();

    let document_format = document_format::DocumentFormat::default();

    let provider = YamlFileProvider::new(String::from("provider.yaml"));

    log::info!("Using yaml file provider");

    let options = Options {
        ..Options::default()
    };

    log::debug!("options: {:?}", options);

    let analysis = {
        if let Ok(stdin) = load_stdin() {
            log::info!("Received data from stdin");
            
            match normalization::normalize_text_to_analysis(
                &provider,
                stdin,
                &Some(options),
            ).await {
                Ok(analysis) => analysis,
                Err(err) => {
                    eprintln!("Failed to normalize text from stdin: {:?}", err);
                    std::process::exit(1);
                }
            }
        } else if let Some(path) = matches.value_of("file") {
            log::info!("Received a file name");

            match normalization::normalize_file_to_analysis(
                &provider,
                path,
                &Some(options),
            ).await {
                Ok(analysis) => analysis,
                Err(err) => {
                    eprintln!("Failed to normalize URL: {:?}", err);
                    std::process::exit(1);
                }
            }
        } else if let Some(url) = matches.value_of("url") {
            log::info!("Received a URL");

            match normalization::normalize_url_to_analysis(
                &provider,
                url,
                &Some(options),
            ).await {
                Ok(analysis) => analysis,
                Err(err) => {
                    eprintln!("Failed to normalize URL: {:?}", err);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("No valid input provided. Please provide either stdin, a file or URL.");
            std::process::exit(1);
        }
    };

    log::info!("Successfully completed analysis");

    match analysis.to_document(&Some(document_format)) {
        Ok(document) => {
            println!("{}", document.to_string());
        },
        Err(err) => {
            eprintln!("Failed to generate normalized document: {:?}", err);
            std::process::exit(1);
        }
    }

    std::process::exit(0);
}
