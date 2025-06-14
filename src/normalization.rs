use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organization::{organize};
use crate::provider::{Provider};
use crate::traverse::{traverse_meta_context};
use crate::meta_context::MetaContext;
use crate::schema::Schema;
use crate::node_analysis::{get_schema_transformations};

#[allow(dead_code)]
pub async fn normalize<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize");

    log::info!("Getting document");
    let document = traverse_meta_context(
        meta_context.clone(),
        &None
    )?;

    println!("document: {}", document.clone().to_string());

    delay();

    unimplemented!();

    {
        let mut lock = write_lock!(meta_context);
        lock.update_document(document);
    }

    log::info!("Getting schema transformations");
    let schema_transformations = get_schema_transformations(
        Arc::clone(&provider),
        Arc::clone(&meta_context)
    ).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_schema_transformations(schema_transformations);
    }

    Ok(meta_context)
}

#[allow(dead_code)]
pub async fn normalize_meta_context<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_meta_context");

    normalize(Arc::clone(&provider), meta_context, _options).await
}

#[allow(dead_code)]
pub async fn normalize_text_to_meta_context<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_text_to_meta_context");

    let document = Document::from_string(text, _options)?;
    let meta_context = organize(Arc::clone(&provider), document, _options).await?;

    normalize_meta_context(Arc::clone(&provider), meta_context, _options).await
}

#[allow(dead_code)]
pub async fn normalize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let meta_context = normalize_text_to_meta_context(Arc::clone(&provider), text, _options).await?;

    traverse_meta_context(
        meta_context,
        document_format,
    )
}

#[allow(dead_code)]
pub async fn normalize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let document = normalize_text_to_document(
        Arc::clone(&provider),
        text,
        _options,
        document_format
    ).await?;

    Ok(document.to_string())
}

#[allow(dead_code)]
pub async fn normalize_document_to_meta_context<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_document_to_meta_context");

    let meta_context = organize(Arc::clone(&provider), document, _options).await?;

    normalize_meta_context(Arc::clone(&provider), meta_context, _options).await
}

#[allow(dead_code)]
pub async fn normalize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let meta_context = normalize_document_to_meta_context(Arc::clone(&provider), document, _options).await?;

    traverse_meta_context(
        meta_context,
        document_format,
    )
}

#[allow(dead_code)]
pub async fn normalize_document_to_text<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(Arc::clone(&provider), document, _options, document_format).await?;

    Ok(document.to_string())
}

#[allow(dead_code)]
pub async fn normalize_file_to_meta_context<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_file_to_meta_context");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path)?;

    normalize_text_to_meta_context(Arc::clone(&provider), text, _options).await
}

#[allow(dead_code)]
pub async fn normalize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let meta_context = normalize_file_to_meta_context(Arc::clone(&provider), path, _options).await?;

    traverse_meta_context(
        meta_context,
        document_format,
    )
}

#[allow(dead_code)]
pub async fn normalize_file_to_text<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(Arc::clone(&provider), path, _options, document_format).await?;

    Ok(document.to_string())
}

#[allow(dead_code)]
pub async fn normalize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<(), Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = normalize_file_to_text(Arc::clone(&provider), path, _options, document_format).await?;
    let new_path = append_to_filename(path, "_normalized")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}

#[allow(dead_code)]
pub async fn normalize_url_to_meta_context<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_url_to_meta_context");
    log::debug!("URL: {}", url);

    let text = fetch_url_as_text(url).await?;

    normalize_text_to_meta_context(Arc::clone(&provider), text, _options).await
}

#[allow(dead_code)]
pub async fn normalize_url_to_document<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_url_to_document");
    log::debug!("URL: {}", url);

    let meta_context = normalize_url_to_meta_context(Arc::clone(&provider), url, _options).await?;

    traverse_meta_context(
        meta_context,
        document_format,
    )
}

#[allow(dead_code)]
pub async fn normalize_url_to_text<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_url_to_text");
    log::debug!("URL: {}", url);

    let document = normalize_url_to_document(Arc::clone(&provider), url, _options, document_format).await?;

    Ok(document.to_string())
}
