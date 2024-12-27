use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organize::{organize};
use crate::analysis::{Analysis};
use crate::model::{Model};
use crate::provider::{Provider};

// TODO: streaming basis graphs, and text

pub async fn normalize<P: Provider>(
    provider: &P,
    analysis: Analysis,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize");

    let basis_graph = analysis.build_basis_graph()?;

    let target_model = Model::get_normal_model(&basis_graph).unwrap();

    analysis.transmute(&target_model.json_schema).await
}

pub async fn normalize_analysis<P: Provider>(
    provider: &P,
    analysis: Analysis,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_analysis");

    normalize(provider, analysis, options).await
}

pub async fn normalize_text_to_analysis<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_text_to_analysis");

    let document = Document::from_string(text, options)?;
    let analysis = organize(provider, document, options).await?;

    normalize_analysis(provider, analysis, options).await
}

pub async fn normalize_text_to_document<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let analysis = normalize_text_to_analysis(provider, text, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_text<P: Provider>(
    provider: &P,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let document = normalize_text_to_document(provider, text, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_document_to_analysis<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_document_to_analysis");

    let analysis = organize(provider, document, options).await?;

    normalize_analysis(provider, analysis, options).await
}

pub async fn normalize_document<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let analysis = normalize_document_to_analysis(provider, document, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_document_to_text<P: Provider>(
    provider: &P,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(provider, document, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file_to_analysis<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_file_to_analysis");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path)?;

    normalize_text_to_analysis(provider, text, options).await
}

pub async fn normalize_file_to_document<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let analysis = normalize_file_to_analysis(provider, path, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_file_to_text<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(provider, path, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file<P: Provider>(
    provider: &P,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<(), Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = normalize_file_to_text(provider, path, options, document_format).await?;
    let new_path = append_to_filename(path, "_normalized")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}

pub async fn normalize_url_to_analysis<P: Provider>(
    provider: &P,
    url: &str,
    options: &Option<Options>,
) -> Result<Analysis, Errors> {
    log::trace!("In normalize_url_to_analysis");
    log::debug!("URL: {}", url);

    let text = fetch_url_as_text(url).await?;

    normalize_text_to_analysis(provider, text, options).await
}

pub async fn normalize_url_to_document<P: Provider>(
    provider: &P,
    url: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_url_to_document");
    log::debug!("URL: {}", url);

    let analysis = normalize_url_to_analysis(provider, url, options).await?;

    analysis.to_document(document_format)
}

pub async fn normalize_url_to_text<P: Provider>(
    provider: &P,
    url: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_url_to_text");
    log::debug!("URL: {}", url);

    let document = normalize_url_to_document(provider, url, options, document_format).await?;

    Ok(document.to_string())
}
