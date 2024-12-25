use crate::prelude::*;
use crate::document::{Document};
use crate::organize::{organize};
use crate::analysis::{Analysis};

pub async fn translate_analysis(
    analysis: Analysis,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_analysis");

    analysis.transmute(json_schema).await
}

pub async fn translate_text_to_analysis(
    text: String,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_text_to_analysis");

    let document = Document::from_string(text, options)?;
    let analysis = organize(document, options).await?;

    translate_analysis(analysis, options, json_schema).await
}

pub async fn translate_text_to_document(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_text_to_document");

    let analysis = translate_text_to_analysis(text, options, json_schema).await?;
    analysis.to_document(document_type)
}

pub async fn translate_text(
    text: String,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_text");

    let document = translate_text_to_document(text, options, document_type, json_schema).await?;
    Ok(document.to_string())
}

pub async fn translate_document_to_analysis(
    document: Document,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_document_to_analysis");

    let analysis = organize(document, options).await?;

    translate_analysis(analysis, options, json_schema).await
}

pub async fn translate_document(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_document");

    let analysis = translate_document_to_analysis(document, options, json_schema).await?
    analysis.to_document(document_type)
}

pub async fn translate_document_to_text(
    document: Document,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_document_to_text");

    let document = translate_document(document, options, document_type, json_schema).await?;

    Ok(document.to_string())
}

pub async fn translate_file_to_analysis(
    path: &str,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {}", err);
        Errors::FileInputError
    })?;

    translate_text_to_analysis(text, options, json_schema).await
}

pub async fn translate_file_to_document(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_file_to_document");

    let analysis = translate_file_to_analysis(path, options, json_schema).await?;
    analysis.to_document(document_type)
}

pub async fn translate_file_to_text(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_file_to_text");

    let document = translate_file_to_document(path, options, document_type, json_schema).await?;

    Ok(document.to_string())
}

pub async fn translate_file(
    path: &str,
    options: &Option<Options>,
    document_type: &Option<DocumentType>,
    json_schema: &str,
) -> Result<(), Errors> {
    log::trace!("In translate_file");
    log::debug!("file path: {}", path);

    let text = translate_file_to_text(path, options, document_type, json_schema).await?;
    let new_path = append_to_filename(path, "_translated")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {}", err);
        Errors::FileOutputError
    })
}
