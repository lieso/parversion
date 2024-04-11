extern crate simple_logging;
extern crate log;

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};

mod models;
mod utilities;
mod tree;
mod llm;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
}

pub fn string_to_json(raw_document: String) -> Result<i8, Errors> {
    log::trace!("In string_to_json");

    if raw_document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let document = raw_document.trim().to_string();

    let _ = Runtime::new().unwrap().block_on(async {

        if utilities::is_valid_html(&document) {
            log::info!("Document is valid HTML");

            let xhtml = utilities::html_to_xhtml(&document).expect("Could not convert HTML to XHTML");

            let json = xml_to_json(xhtml).await?;

            return Ok(json);
        }

        if utilities::is_valid_xml(&document) {
            log::info!("Document is valid XML");

            let json = xml_to_json(document).await?;

            return Ok(json);
        }

        Err(Errors::UnexpectedDocumentType)
    });

    Err(Errors::UnexpectedError)
}

pub async fn xml_to_json(xml: String) -> Result<i8, Errors> {
    log::trace!("In xml_to_json");

    let xml = utilities::preprocess_xml(&xml);

    let t = tree::build_tree(xml);

    let t = tree::grow_tree(&mut t.clone()).await;

    Ok(1)
}

pub fn file_to_json(file_name: &str) -> Result<i8, Errors> {
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
