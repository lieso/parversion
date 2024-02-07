extern crate simple_logging;
extern crate log;

use serde::Serialize;
use tokio::runtime::Runtime;
use std::fs::{OpenOptions, File};
use std::process;
use std::io::{Read};
use std::io::{self};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use std::io::Write;

mod prompts {
    pub mod chat;
    pub mod list;
}
mod utilities;
mod parsers {
    pub mod chat;
    pub mod list;
}
mod models {
    pub mod chat;
    pub mod list;
}
mod transformers {
    pub mod chat;
    pub mod list;
}

#[derive(Debug, Serialize)]
struct Output<T, U> {
    parsers: Vec<T>,
    data: Vec<U>,
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

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn save_parser_to_file<T>(parser: &T) where T: Serialize {
    log::trace!("In save_parser_to_file");

    let serialized = serde_json::to_string_pretty(parser).expect("Serialization failed");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("parsers.json")
        .expect("Failed to open file");

    file.write_all(serialized.as_bytes()).expect("Failed to write to file");
    file.write_all("\n".as_bytes()).expect("Failed to write to file");
}

fn document_to_chat(document: String) {
    log::trace!("In document_to_chat");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let parser = parsers::chat::get_chat_parser(sample).await.unwrap();
        log::debug!("parser: {:?}", parser);

        save_parser_to_file(&parser);

        let chat: models::chat::Chat = transformers::chat::transform_document_to_chat(document, parser);
        log::debug!("chat: {:?}", chat);

        let output = serde_json::to_string(&chat).expect("Failed to serialize to JSON");
        println!("{}", output);
    });
}

fn document_to_list(document: String) {
    log::trace!("In document_to_list");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let parsers = parsers::list::get_list_parser(sample).await.unwrap();
        log::debug!("parsers: {:?}", parsers);

        let mut output = Output {
            parsers: parsers.clone(),
            data: Vec::new(),
        };

        for parser in parsers.iter() {
            let list: models::list::List = transformers::list::transform_document_to_list(document.clone(), &parser);
            log::debug!("list: {:?}", list);

            output.data.push(list);
        }

        let serialized = serde_json::to_string(&output).expect("Failed to serialize to JSON");
        println!("{}", serialized);
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
            "list" => document_to_list(document),
            _ => log::error!("Unexpected data type: {}", data_type),
        }
        return;
    } else {
        log::info!("Data type not provided, aborting...");
        return;
    }
}
