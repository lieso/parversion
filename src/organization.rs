use std::sync::Arc;

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::provider::Provider;
use crate::traverse::{
    traverse_with_context,
    build_document_from_meta_context
};
use crate::analysis::{Analysis};
use crate::meta_context::MetaContext;

pub async fn organize<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
) -> Result<Arc<MetaContext>, Errors> {
    log::trace!("In organize");

    let mut document = document;

    let profile = document.perform_analysis(provider.clone()).await?;

    let meta_context = traverse_with_context(&profile, document)
        .expect("Could not traverse document");

    let meta_context = Arc::new(meta_context);

    Analysis::start(
        Arc::clone(&provider),
        Arc::clone(&meta_context)
    ).await?;

    Ok(meta_context)
}

pub async fn organize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_document");

    let meta_context = organize(
        Arc::clone(&provider),
        document,
        options
    ).await?;

    build_document_from_meta_context(
        provider,
        meta_context,
        document_format,
    ).await
}

pub async fn organize_document_to_string<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In organize_document_to_string");

    let document = organize_document(
        Arc::clone(&provider),
        document,
        options,
        document_format
    ).await?;

    Ok(document.to_string())
}

pub async fn organize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
) -> Result<Arc<MetaContext>, Errors> {
    log::trace!("In organize_text");

    let document = Document::from_string(text, options)?;

    organize(Arc::clone(&provider), document, options).await
}

pub async fn organize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_text_to_document");

    let meta_context = organize_text(Arc::clone(&provider), text, options).await?;

    build_document_from_meta_context(provider, meta_context, document_format).await
}

pub async fn organize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
) -> Result<Arc<MetaContext>, Errors> {
    log::trace!("In organize_file");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    organize_text(Arc::clone(&provider), text, options).await
}

pub async fn organize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_file_to_document");
    log::debug!("file path: {}", path);

    let meta_context = organize_file(Arc::clone(&provider), path, options).await?;

    build_document_from_meta_context(provider, meta_context, document_format).await
}

pub async fn organize_file_to_string<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In organize_file_to_string");
    log::debug!("file path: {}", path);

    let document = organize_file_to_document(Arc::clone(&provider), path, options, document_format).await?;

    Ok(document.to_string())
}
