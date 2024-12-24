use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};

use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::analysis::{Analysis};

pub async fn organize_file(
    file_name: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_file");
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

    organize_text(text, options).await
}

pub async fn organize_text(
    text: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_text");

    let document = Document::from_string(text, options)?;

    organize_document(document, options).await
}

pub async fn organize_document(
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_document");

    let basis_graph = options.as_ref().and_then(|opts| opts.basis_graph.clone());

    let value_transformations = options
        .as_ref()
        .and_then(|opts| opts.value_transformations.clone())
        .unwrap_or_else(Vec::new);

    Analysis::from_document(document, &options)
        .with_basis(basis_graph)
        .with_value_transformations(value_transformations)
        .perform_analysis()
        .await?
}
