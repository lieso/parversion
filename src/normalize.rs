use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};

use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::organize::{organize_document};

pub struct Normalization {
    pub basis_graph: BasisGraph,
    pub related_data: OutputData,
    pub normalized_data: OutputData,
}

pub async fn normalize_file(
    file_name: String,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
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
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, options)?;

    document.perform_document_analysis().await;
    document.apply_document_transformations();

    normalize_document(document, options).await
}

pub async fn normalize_document(
    document: Document,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document");

    let organization = organize::organize_document(document, options);

    normalize_organization(organization, options).await
}

pub async fn normalize_organization(
    organization: Organization,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_organization");

    let Organization {
        basis_graph,
        organized_data,
        related_data
    } = organization;

    let normalized_data = basis_graph.normalize(organized_data).await;

    let normalization = Normalization {
        basis_graph,
        related_data,
        normalized_data,
    };

    Ok(normalization)
}
