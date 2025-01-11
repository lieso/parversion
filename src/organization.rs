use std::sync::Arc;

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::analysis::{Analysis, AnalysisInput};
use crate::provider::Provider;

pub async fn organize<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize");

    let input = AnalysisInput::from_document(Arc::clone(&provider), document).await?;
    let analysis = Analysis::new(Arc::clone(&provider), input).await?;

    Ok(analysis)
}

pub async fn organize_document_to_analysis<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_document_to_analysis");

    organize(Arc::clone(&provider), document, options).await
}

pub async fn organize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_document");

    let analysis = organize_document_to_analysis(Arc::clone(&provider), document, options).await?;

    analysis.to_document(document_format)
}

pub async fn organize_document_to_text<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In organize_document_to_text");

    let document = organize_document(Arc::clone(&provider), document, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn organize_text_to_analysis<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_text_to_analysis");

    let document = Document::from_string(text, options)?;

    organize_document_to_analysis(Arc::clone(&provider), document, options).await
}

pub async fn organize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_text_to_document");

    let analysis = organize_text_to_analysis(Arc::clone(&provider), text, options).await?;

    analysis.to_document(document_format)
}

pub async fn organize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In organize_text");

    let document = organize_text_to_document(Arc::clone(&provider), text, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn organize_file_to_analysis<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In organize_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    organize_text_to_analysis(Arc::clone(&provider), text, options).await
}

pub async fn organize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_file_to_document");
    log::debug!("file path: {}", path);

    let analysis = organize_file_to_analysis(Arc::clone(&provider), path, options).await?;

    analysis.to_document(document_format)
}

pub async fn organize_file_to_text<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In organize_file_to_text");
    log::debug!("file path: {}", path);

    let document = organize_file_to_document(Arc::clone(&provider), path, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn organize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<(), Errors> {
    log::trace!("In organize_file");
    log::debug!("file path: {}", path);

    let text = organize_file_to_text(Arc::clone(&provider), path, options, document_format).await?;
    let new_path = append_to_filename(path, "_organized")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write organized text to file: {:?}", err);
        Errors::FileOutputError
    })
}
