extern crate simple_logging;
extern crate log;

use serde::{Serialize};
use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};

pub mod parsers;
pub mod models;
pub mod transformers;
pub mod prompts;
pub mod utilities;

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    IncorrectParser,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
pub enum Document {
    Chat(models::chat::Chat),
    List(models::list::List),
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Parser {
    Chat(models::chat::ChatParser),
    List(models::list::ListParser),
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
pub enum ParserType {
    Chat,
    List,
}

#[derive(Debug)]
#[derive(Serialize)]
pub struct Output {
    pub parsers: Vec<Parser>,
    pub data: Vec<Document>,
}

pub fn string_to_json(document: String, document_type: &str) -> Result<Output, Errors> {
    log::trace!("In string_to_json");
    log::debug!("document_type: {}", document_type);

    if document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let parsers = get_parsers(document.clone(), document_type).await?;

        return get_output(document.clone(), document_type, &parsers);
    })
}

pub fn get_output(document: String, document_type: &str, parsers: &Vec<Parser>) -> Result<Output, Errors> {
    log::trace!("In get_output");
    
    let mut output = Output {
        parsers: parsers.clone(),
        data: Vec::new(),
    };

    for parser in parsers.iter() {
        let result = parse_document(document.clone(), document_type, parser.clone())?;
        output.data.push(result);
    }

    Ok(output)
}

pub fn file_to_json(file_name: &str, document_type: &str) -> Result<Output, Errors> {
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

pub fn parse_document(document: String, document_type: &str, parser: Parser) -> Result<Document, Errors> {
    log::trace!("In parse_text");
    log::debug!("document_type: {}", document_type);

    match document_type {
        "chat" => {
            if let Parser::Chat(chat_parser) = parser {
                let chat = transformers::chat::transform_document_to_chat(document.clone(), &chat_parser);
                Ok(Document::Chat(chat))
            } else {
                Err(Errors::IncorrectParser)
            }
        }
        "list" => {
            if let Parser::List(list_parser) = parser {
                let list = transformers::list::transform_document_to_list(document.clone(), &list_parser);
                Ok(Document::List(list))
            } else {
                Err(Errors::IncorrectParser)
            }
        }
        _ => {
            Err(Errors::UnexpectedDocumentType)
        }
    }
}

pub async fn get_parsers(document: String, document_type: &str) -> Result<Vec<Parser>, Errors> {
    log::trace!("In get_parsers");
    log::debug!("document_type: {}", document_type);

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    match document_type {
        "chat" => {
            let chat_parsers = parsers::chat::get_chat_parser(sample).await;
            if let Ok(ok_chat_parsers) = chat_parsers {
                log::info!("Obtained chat parsers without errors");

                let parsers: Vec<Parser> = ok_chat_parsers
                    .iter()
                    .map(|parser| {
                        Parser::Chat(parser.clone())
                    })
                    .collect();

                Ok(parsers)
            } else {
                Err(Errors::UnexpectedError)
            }
        }
        "list" => {
            let list_parsers = parsers::list::get_list_parser(sample).await;

            if let Ok(ok_list_parsers) = list_parsers {
                log::info!("Obtained list parsers without errors");

                let parsers: Vec<Parser> = ok_list_parsers
                    .iter()
                    .map(|parser| {
                        Parser::List(parser.clone())
                    })
                    .collect();

                Ok(parsers)
            } else {
                Err(Errors::UnexpectedError)
            }
        }
        _ => {
            Err(Errors::UnexpectedDocumentType)
        }
    }
}

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
