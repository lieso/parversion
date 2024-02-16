extern crate simple_logging;
extern crate log;

use serde::{Serialize, Value};
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

pub fn string_to_json(document: String, document_type: String) -> Result<Output> {
    log::trace!("In string_to_json");
    log::debug!("document_type: {}", document_type);

    if document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err();
    }

    let parsers = get_parsers(document, document_type);

    let mut output = Output {
        parsers: parsers.clone(),
        data: Vec::new(),
    };

    for parser in parsers.iter() {
        let result = parse_document(document, document_type, &parser);
        output.data.push(result);
    }
    
    return output;
}

pub fn file_to_json(file_name: String, document_type: String) -> Result<Output> {
    log::trace!("In file_to_json");
    log::debug!("file_name: {}", file_name);
    log::debug!("document_type: {}", document_type);

    let mut document = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut document).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    return string_to_json(document, document_type);
}

pub fn parse_document(document: String, document_type: String, parser: Value) -> Result<Value> {
    log::trace!("In parse_text");
    log::debug!("document_type: {}", document_type);

    match document_type {
        "chat" => {
            return transformers::chat::transform_document_to_chat(document.clone(), &parser);
        }
        "list" => {
            return transformers::list::transform_document_to_list(document.clone(), &parser);
        }
        _ => {
            panic!("Unexpected document type");
        }
    }
}

pub fn get_parsers(document: String, document_type: String) -> Result<Value> {
    log::trace!("In get_parsers");
    log::debug!("document_type: {}", document_type);

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        match document_type {
            "chat" => {
                return parsers::list::get_chat_parser(sample).await.unwrap();
            }
            "list" => {
                return parsers::list::get_list_parser(sample).await.unwrap();
            }
            _ => {
                panic!("Unexpected document type");
            }
        }
    });
}

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
