use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organize::{organize};
use crate::analysis::{Analysis};
use crate::model::{Model};

pub async fn normalize(
    analysis: Analysis,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_analysis");

    let basis_graph = analysis.build_basis_graph()?;

    let target_model = Model::get_normal_json_schema(&basis_graph).unwrap();

    analysis.transmute(&target_model.json_schema).await
}

pub async fn normalize_analysis(
    analysis: Analysis,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_analysis");

    normalize(analysis, options).await
}

pub async fn normalize_text_to_analysis(
    text: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_text_to_analysis");

    let document = Document::from_string(text, options)?;
    let analysis = organize(document, options).await?;

    normalize_analysis(analysis, options).await
}

pub async fn normalize_text_to_document(
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let analysis = normalize_text_to_analysis(text, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_text(
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let document = normalize_text_to_document(text, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_document_to_analysis(
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_document_to_analysis");

    let analysis = organize(document, options).await?;

    normalize_analysis(analysis, options).await
}

pub async fn normalize_document(
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let analysis = normalize_document_to_analysis(document, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_document_to_text(
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(document, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file_to_analysis(
    path: &str,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path)?;

    normalize_text_to_analysis(text, options).await
}

pub async fn normalize_file_to_document(
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let analysis = normalize_file_to_analysis(path, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_file_to_text(
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(path, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file(
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<(), Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = normalize_file_to_text(path, options, document_format).await?;
    let new_path = append_to_filename(path, "_normalized")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}
