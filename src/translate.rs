use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};
use std::path::Path;

use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::organize;
use crate::analysis::{Analysis};

pub async fn translate_file(
    file_name: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_file");
    log::debug!("file_name: {}", file_name);

    let mut text = String::new();

    let mut file = File::open(file_name).map_err(|err| {
        log::error!("Failed to open file: {}", err);
        Errors::FileInputError
    })?;

    file.read_to_string(&mut text).map_err(|err| {
        log::error!("Failed to read file: {}", err);
        Errors::FileInputError
    })?;

    translate_text(text, options, json_schema).await
}

pub async fn translate_text(
    text: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_text");

    let document = Document::from_string(text, options)?;

    translate_document(document, options, json_schema).await
}

pub async fn translate_document(
    document: Document
    options: Option<Options>,
    json_schema: String,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_document");

    let analysis = organize::organize_document(document, options);

    translate_analysis(analysis, options, json_schema).await
}

pub async fn translate_analysis(
    analysis: Analysis,
    options: Option<Options>,
    json_schema: String,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_document");

    analysis.get_schema_transformations(json_schema)
        .await
        .apply_schema_transformations()?
}
