use std::io::{Read};
use std::fs::File;
use std::path::Path;
use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::analysis::{Analysis};

pub async fn organize_text_to_document(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<Document, Errors> {
    log::trace!("In organize_text_to_document");

    let document = Document::from_string(text, options)?;
    organize_document(document, options, document_type).await
}

pub async fn organize_text(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In organize_text");

    let document = organize_text_to_document(text, options, document_type).await?;
    document.to_string()
}

pub async fn organize_document(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<Document, Errors> {
    log::trace!("In organize_document");

    let analysis = Analysis::from_document(document, options).perform_analysis().await?;
    analysis.to_document(document_type)
}

pub async fn organize_document_to_text(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In organize_document_to_text");

    let document = organize_document(document, options, document_type).await?;
    document.to_string()
}

pub async fn organize_file_to_document(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<Document, Errors> {
    log::trace!("In organize_file_to_document");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {}", err);
        Errors::FileInputError
    })?;
    organize_text_to_document(text, options, document_type).await
}

pub async fn organize_file_to_text(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In organize_file_to_text");
    log::debug!("file path: {}", path);

    let document = organize_file_to_document(path, options, document_type).await?;
    document.to_string()
}

pub async fn organize_file(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<(), Errors> {
    log::trace!("In organize_file");
    log::debug!("file path: {}", path);

    let text = organize_file_to_text(path, options, document_type).await?;
    let new_path = append_to_filename(path, "_organized")?;

    write_text_to_file(new_path, text).map_err(|err| {
        log::error!("Failed to write organized text to file: {}", err);
        Errors::FileOutputError
    })
}
