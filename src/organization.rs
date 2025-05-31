use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::provider::Provider;
use crate::traverse::{
    traverse_document,
    build_document_from_meta_context
};
use crate::meta_context::MetaContext;
use crate::node_analysis::{get_basis_nodes};
use crate::network_analysis::{get_basis_networks, get_basis_graph};

#[allow(dead_code)]
pub async fn organize<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize");

    let meta_context = Arc::new(RwLock::new(MetaContext::new()));

    log::info!("Performing document analysis");
    let profile = document.perform_analysis(Arc::clone(&provider)).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_profile(profile);
    }

    log::info!("Traversing document using profile");
    let (contexts, graph_root) = traverse_document(document, meta_context.clone())?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_data_structures(contexts, graph_root);
    }

    log::info!("Getting basis graph");
    let basis_graph = get_basis_graph(
        Arc::clone(&provider),
        meta_context.clone(),
    ).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_graph(basis_graph);
    }

    log::info!("Getting basis nodes");
    let basis_nodes = get_basis_nodes(
        Arc::clone(&provider),
        meta_context.clone(),
    ).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_nodes(basis_nodes);
    }

    log::info!("Generating basis networks");
    let basis_networks = get_basis_networks(
        Arc::clone(&provider),
        meta_context.clone(),
    ).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_networks(basis_networks);
    }

    Ok(meta_context)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub async fn organize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize_text");

    let document = Document::from_string(text, options)?;

    organize(Arc::clone(&provider), document, options).await
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub async fn organize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize_file");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    organize_text(Arc::clone(&provider), text, options).await
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
