use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};

use crate::basis_graph::BasisGraph;

pub struct Organization {
    pub basis_graph: BasisGraph,
    pub related_data: Option<OutputData>,
    pub organized_data: OutputData,
}

pub async fn organize_file(
    file_name: String,
    options: Option<Options>,
) -> Result<Organization, Errors> {
    log::trace!("In organize_file");
    log::debug!("file_name: {}", file_name);

    let mut text = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut text).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    organize_text(text, options).await
}

pub async fn organize_text(
    text: String,
    options: Option<Options>,
) -> Result<Organization, Errors> {
    log::trace!("In organize_text");

    let document = Document::from_string(text)?;

    document.perform_document_analysis().await;
    document.apply_document_transformations();

    organize_document(document, options).await
}

pub async fn organize_document(
    document: Document,
    options: Option<Options>,
) -> Result<Organization, Errors> {
    log::trace!("In organize_text");

    let basis_graph = options
        .and_then(|opts| opts.basis_graph)
        .unwrap_or_else(|| classify_or_create_basis_graph(document));

    let analysis = Analysis::from_document(document)
        .with_basis(basis_graph)
        .perform_analysis()
        .await;

    analysis.apply_value_transformations(value_transformations);

    Organization {
        basis_graph: analysis.get_basis_graph(),
        organized_data: analysis.get_data(),
        related_data: analysis.get_related_data(),
    }
}
