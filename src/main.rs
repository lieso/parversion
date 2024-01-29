extern crate simple_logging;
extern crate log;

use tokio::runtime::Runtime;
use std::fs::File;
use std::process;
use std::io::{Read};
use std::io::{self};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;

mod prompts;
mod utilities;
mod parsers;
mod models;
mod transformers;

fn load_stdin() -> io::Result<String> {
    log::trace!("In load_stdin");

    if atty::is(Stream::Stdin) {
        return Err(io::Error::new(io::ErrorKind::Other, "stdin not redirected"));
    }
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn document_to_chat(document: String) {
    log::trace!("In document_to_chat");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let parser = parsers::get_chat_parser(sample).await.unwrap();
        log::debug!("parser: {:?}", parser);

        let chat: models::Chat = transformers::transform_document_to_chat(document, parser);
        log::debug!("chat: {:?}", chat);

        let output = serde_json::to_string(&chat).expect("Failed to serialize to JSON");
        println!("{}", output);
    });
}

fn main() {
    log::trace!("In main");

    let _ = simple_logging::log_to_file("debug.log", LevelFilter::Trace);

    let mut document = String::new();

    match load_stdin() {
        Ok(stdin) => {
            document = stdin;
        }
        Err(_e) => {
            log::debug!("Did not receive input from stdin");
        }
    }

    let matches = App::new("document-to-json")
        .arg(Arg::with_name("type")
             .short('t')
             .long("type")
             .value_name("TYPE")
             .required(true))
        .arg(Arg::with_name("file")
             .short('f')
             .long("file")
             .value_name("FILE")
             .help("Provide file as document for processing"))
        .get_matches();

    if let Some(file_name) = matches.value_of("file") {
        log::debug!("file_name: {}", file_name);
        let mut file = File::open(file_name).unwrap_or_else(|err| {
            eprintln!("Failed to open file: {}", err);
            process::exit(1);
        });

        file.read_to_string(&mut document).unwrap_or_else(|err| {
            eprintln!("Failed to read file: {}", err);
            process::exit(1);
        });
    } else {
        log::debug!("File not provided");
    }

    if document.trim().is_empty() {
        log::debug!("Document not provided, aborting...");
        return;
    }

    if let Some(data_type) = matches.value_of("type") {
        log::debug!("data_type: {}", data_type);

        match data_type {
            "chat" => document_to_chat(document),
            _ => log::error!("Unexpected data type: {}", data_type),
        }
        return;
    } else {
        log::info!("Data type not provided, aborting...");
        return;
    }
}
