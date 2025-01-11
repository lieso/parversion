use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organization::organize;
use crate::analysis::{Analysis};
use crate::provider::Provider;

pub async fn translate<P: Provider>(
    provider: &P,
    analysis: Analysis,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate");

    unimplemented!()
}

pub async fn translate_analysis<P: Provider>(
    provider: &P,
    analysis: Analysis,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_analysis");

    translate(provider, analysis, options, json_schema).await
}

pub async fn translate_text_to_analysis<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_text_to_analysis");

    let document = Document::from_string(text, options)?;
    let analysis = organize(provider, document, options).await?;

    translate_analysis(provider, analysis, options, json_schema).await
}

pub async fn translate_text_to_document<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_text_to_document");

    let analysis = translate_text_to_analysis(provider, text, options, json_schema).await?;

    analysis.to_document(document_format)
}

pub async fn translate_text<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_text");

    let document = translate_text_to_document(
        provider,
        text,
        options,
        document_format,
        json_schema
    ).await?;

    Ok(document.to_string())
}

pub async fn translate_document_to_analysis<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_document_to_analysis");

    let analysis = organize(provider, document, options).await?;

    translate_analysis(provider, analysis, options, json_schema).await
}

pub async fn translate_document<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_document");

    let analysis = translate_document_to_analysis(
        provider,
        document,
        options,
        json_schema
    ).await?;

    analysis.to_document(document_format)
}

pub async fn translate_document_to_text<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_document_to_text");

    let document = translate_document(
        provider,
        document,
        options,
        document_format,
        json_schema
    ).await?;

    Ok(document.to_string())
}

pub async fn translate_file_to_analysis<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    json_schema: &str,
) -> Result<Analysis, Errors> {
    log::trace!("In translate_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    translate_text_to_analysis(provider, text, options, json_schema).await
}

pub async fn translate_file_to_document<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_file_to_document");

    let analysis = translate_file_to_analysis(provider, path, options, json_schema).await?;
    analysis.to_document(document_format)
}

pub async fn translate_file_to_text<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<String, Errors> {
    log::trace!("In translate_file_to_text");

    let document = translate_file_to_document(
        provider,
        path,
        options,
        document_format,
        json_schema
    ).await?;

    Ok(document.to_string())
}

pub async fn translate_file<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
    json_schema: &str,
) -> Result<(), Errors> {
    log::trace!("In translate_file");
    log::debug!("file path: {}", path);

    let text = translate_file_to_text(provider, path, options, document_format, json_schema).await?;
    let new_path = append_to_filename(path, "_translated")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}
