use std::io::{Read};
use std::fs::File;
use serde_json::{Value};

use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::organize::{organize_document};
use crate::analysis::{Analysis};

pub async fn normalize_file(
    file_name: String,
    options: Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_file");
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

    normalize_text(text, options).await
}

pub async fn normalize_text(
    text: String,
    options: Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, options)?;

    normalize_document(document, options).await
}

pub async fn normalize_document(
    document: Document,
    options: Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_document");

    let analysis = organize::organize_document(document, options);

    normalize_analysis(analysis, options).await
}

pub async fn normalize_analysis(
    analysis: Analysis,
    options: Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_analysis");

    let basis_graph = analysis.get_basis_graph();

    let target_schema = get_normal_json_schema(&basis_graph);

    analysis.get_schema_transformations(target_schema)
        .await
        .apply_schema_transformations()?
}
