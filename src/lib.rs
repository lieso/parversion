extern crate simple_logging;
extern crate log;

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use pandoculation;
use html_escape;

pub mod parsers;
pub mod models;
pub mod transformers;
pub mod prompts;
pub mod utilities;
pub mod categorisers;
pub mod adapters;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnableToCategoriseDocument,
    SalientContentNotFound,
    LlmRequestError
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Document {
    List(models::list::List),
    Chat(pandoculation::Chat),
    CuratedListing(pandoculation::CuratedListing),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Parser {
    Chat(models::chat::ChatParser),
    List(models::list::ListParser),
    CuratedListing(models::curated_listing::CuratedListingParser),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ParserType {
    Chat,
    List,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub parsers: Vec<Parser>,
    pub data: Vec<Document>,
}

pub fn string_to_json(raw_document: String) -> Result<Output, Errors> {
    log::trace!("In string_to_json");

    if raw_document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let rt = Runtime::new().unwrap();

    let document: String = preprocess_document(raw_document);

    rt.block_on(async {
        const DOCUMENT_SAMPLE_SIZE: usize = 20000;

        let chunks = utilities::text::chunk_string(&document, DOCUMENT_SAMPLE_SIZE);
        log::debug!("number of chunks: {}", chunks.len());

        let sample = get_salient_sample(chunks).await?;

        if let Ok(document_types) = categorisers::get_document_types(&sample).await {
            log::debug!("document_types: {:?}", document_types);

            let first_document_type = document_types.first().expect("Unable to categorise document");
            let parsers = get_parsers(&document, &sample, &first_document_type).await?;

            return get_output(document.clone(), &parsers);
        } else {
            return Err(Errors::UnableToCategoriseDocument);
        }
    })
}

pub fn get_output(document: String, parsers: &Vec<Parser>) -> Result<Output, Errors> {
    log::trace!("In get_output");
    
    let mut output = Output {
        parsers: parsers.clone(),
        data: Vec::new(),
    };

    for parser in parsers.iter() {
        let result = parse_document(&document, &parser)?;

        log::info!("Completed parsing document");

        output.data.push(result);
    }

    Ok(output)
}

pub fn file_to_json(file_name: &str) -> Result<Output, Errors> {
    log::trace!("In file_to_json");
    log::debug!("file_name: {}", file_name);

    let mut document = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut document).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    return string_to_json(document);
}

pub fn parse_document(document: &str, parser: &Parser) -> Result<Document, Errors> {
    log::trace!("In parse_text");

    match parser {
        Parser::Chat(chat_parser) => {
            let chat = transformers::chat::transform(document.to_string(), chat_parser);
            Ok(Document::Chat(chat))
        }
        Parser::CuratedListing(curated_listing_parser) => {
            let curated_list = transformers::curated_listing::transform(document.to_string(), curated_listing_parser);
            Ok(Document::CuratedListing(curated_list))
        }
        _ => {
            Err(Errors::UnexpectedDocumentType)
        }
    }
}

pub async fn get_parsers(
    document: &str,
    sample: &str,
    document_type: &models::document_type::DocumentType,
) -> Result<Vec<Parser>, Errors> {
    log::trace!("In get_parsers");
    log::debug!("document_type: {:?}", document_type);

    match document_type {
        models::document_type::DocumentType::Chat => {
            let chat_parsers = parsers::chat::get_parsers(document, sample).await;

            if let Ok(chat_parsers) = chat_parsers {
                log::info!("Obtained chat parsers without errors");

                let parsers: Vec<Parser> = chat_parsers
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
        models::document_type::DocumentType::CuratedListing => {
            let curated_listing_parsers = parsers::curated_listing::get_parsers(document, sample).await;

            if let Ok(curated_listing_parsers) = curated_listing_parsers {
                log::info!("Obtained curated listing parsers without errors");

                let parsers: Vec<Parser> = curated_listing_parsers
                    .iter()
                    .map(|parser| {
                        Parser::CuratedListing(parser.clone())
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

pub async fn get_salient_sample(chunks: Vec<String>) -> Result<String, Errors> {
    log::trace!("In get_salient_sample");

    for (chunk_index, chunk) in chunks.iter().enumerate() {
        log::debug!("chunk_index: {}", chunk_index);

        let prompt = format!("{} {}", prompts::salient_index::SALIENT_INDEX, &chunk);
        let llm_response = utilities::llm::get_llm_response(prompt).await;

        match llm_response {
            Ok(response) => {
                log::info!("Success response from llm");
                log::debug!("response: {:?}", response);

                let json = response
                    .as_object()
                    .unwrap();

                let status = &json["status"];
                log::debug!("status: {}", status);

                if status.to_string().to_lowercase() == "\"success\"" {
                    log::debug!("Salient index found");

                    let content_index = &json["content_index"];
                    log::debug!("content_index: {}", content_index);

                    let content_index: usize = content_index.as_u64().expect("Expected a usize") as usize;

                    let current_chunk_salient_content = &chunk[content_index..];
                    log::debug!("current_chunk_salient_content: {}", current_chunk_salient_content);

                    match chunks.get(chunk_index + 1) {
                        Some(next_chunk) => {
                            let sample = current_chunk_salient_content.to_owned() + next_chunk;
                            return Ok(sample);
                        },
                        None => {
                            return Ok(current_chunk_salient_content.to_string());
                        }
                    }
                } else {
                    log::debug!("Salient index not found");

                    let message = &json["message"];
                    log::debug!("message: {}", message);
                }
            }
            Err(error) => {
                log::error!("{}", error);
                return Err(Errors::LlmRequestError);
            }
        }
    }

    Err(Errors::SalientContentNotFound)
}

pub fn preprocess_document(document: String) -> String {
    log::trace!("In preprocess_document");

    html_escape::decode_html_entities(&document).into_owned()
}

