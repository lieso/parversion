use crate::prelude::*;
use crate::basis_graph::{BasisGraph};
use crate::document::{Document};
use crate::types::*;
use crate::organize::{organize_document};
use crate::analysis::{Analysis};
use crate::model;

pub async fn normalize_analysis(
    analysis: Analysis,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_analysis");

    let basis_graph = analysis.get_basis_graph();

    let target_schema = model::get_normal_json_schema(&basis_graph);

    analysis.transmute(target_schema).await?
}

pub async fn normalize_text_to_analysis(
    text: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_text_to_analysis");

    let document = Document::from_string(text, options)?;
    let analysis = organize_document(document, options).await?;

    normalize_analysis(analysis, options).await?
}

pub async fn normalize_text_to_document(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let analysis = normalize_text_to_analysis(text, options)?;

    analysis.to_document(document_type)
}

pub async fn normalize_text(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let document = normalize_text_to_document(text, options, document_type).await?;

    document.to_string()
}

pub async fn normalize_document_to_analysis(
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_document_to_analysis");

    let analysis = organize::organize_document(document, options)?;

    normalize_analysis(analysis, options).await?;
}

pub async fn normalize_document(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let analysis = normalize_document_to_analysis(document, options)?;

    analysis.to_document(document_type)
}

pub async fn normalize_document_to_text(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(document, options, document_type)?;

    document.to_string()
}

pub async fn normalize_file_to_analysis(
    path: &str,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path)?;

    normalize_text_to_analysis(text, options).await?
}

pub async fn normalize_file_to_document(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let analysis = normalize_file_to_analysis(path, options)?;

    analysis.to_document(document_type)
}

pub async fn normalize_file_to_text(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(path, options, document_type)?;

    document.to_string()
}

pub async fn normalize_file(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
) -> Result<(), Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = normalize_file_to_text(path, options, document_type)?;

    let new_path = append_to_filename(path, "_normalized")?;

    // TODO: both params are strings, order may be confused
    write_text_to_file(new_path, text)?;

    Ok(())
}
