use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};
use std::path::Path;
use tokio::task;

use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::organize;

pub struct Translation {
    pub basis_graph: BasisGraph,
    pub related_data: OutputData,
    pub translated_data: OutputData,
}

pub async fn translate_file(
    file_name: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_file");
    log::debug!("file_name: {}", file_name);

    let text = task::spawn_blocking(move || -> Result<String, Errors> {
        let path = Path::new(&file_name);
        let mut file = File::open(path).map_err(|err| {
            log::error!("Failed to open file: {}", err);
            Errors::FileInputError
        })?;

        let mut text = String::new();
        file.read_to_string(&mut text).map_err(|err| {
            log::error!("Failed to read file: {}", err);
            Errors::FileInputError
        })?;

        Ok(text)
    }).await.map_err(|_| Errors::FileInputError)?;

    translate_text(text?, options, json_schema).await
}

pub async fn translate_text(
    text: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_text");

    let document = Document::from_string(text, options)?;

    document.perform_document_analysis().await;
    document.apply_document_transformations();

    translate_document(document, options, json_schema).await
}

pub async fn translate_document(
    document: Document
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_document");

    let organization = organize::organize_document(document, options);

    translate_organization(organization, options, json_schema).await
}

pub async fn translate_organization(
    organization: Organization,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_document");

    let Organization {
        basis_graph,
        organized_data,
        related_data
    } = organization;

    let translated_data = basis_graph.translate(organized_data, json_schema).await;

    let translation = Translation {
        basis_graph,
        related_data,
        translated_data,
    };

    Ok(translation)
}
