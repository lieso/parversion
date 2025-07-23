use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::node_analysis::{get_basis_nodes};
use crate::network_analysis::{get_basis_networks, get_basis_graph};
use crate::document_format::DocumentFormat;

#[allow(dead_code)]
pub async fn organize<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize");

    let meta_context = Arc::new(RwLock::new(MetaContext::new()));

    {
        let mut lock = write_lock!(meta_context);
        lock.add_document_version(DocumentVersion::InputDocument, document.clone());
    }

    log::info!("Performing document analysis");
    let profile = document.perform_analysis(Arc::clone(&provider)).await?;
    let profile = Arc::new(profile);

    {
        let mut lock = write_lock!(meta_context);
        lock.update_profile(profile);
    }

    log::info!("Traversing document");
    let (contexts, graph_root) = document.get_contexts(meta_context.clone())?;

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

    {
        let organized = Document::from_basis_transformations(Arc::clone(&meta_context))?;
        let result = format!("{}", organized.to_string(&None));
        log::debug!("\n\n\
        =======================================================\n\
        =============   ORGANIZED DOCUMENT START   =================\n\
        =======================================================\n\
        {}
        =======================================================\n\
        =============    ORGANIZED DOCUMENT END    =================\n\
        =======================================================\n\n", result);
    }

    Ok(meta_context)
}

#[allow(dead_code)]
pub async fn organize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
) -> Result<Document, Errors> {
    log::trace!("In organize_document");

    let meta_context = organize(
        Arc::clone(&provider),
        document,
        _options
    ).await?;

    let organized_document = Document::from_basis_transformations(Arc::clone(&meta_context));

    organized_document
}

#[allow(dead_code)]
pub async fn organize_document_to_string<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In organize_document_to_string");

    let document = organize_document(
        Arc::clone(&provider),
        document,
        _options,
    ).await?;

    Ok(document.to_string(document_format))
}

#[allow(dead_code)]
pub async fn organize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize_text");

    let document = Document::from_string(text, _options)?;

    organize(Arc::clone(&provider), document, _options).await
}

#[allow(dead_code)]
pub async fn organize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
) -> Result<Document, Errors> {
    log::trace!("In organize_text_to_document");

    let meta_context = organize_text(Arc::clone(&provider), text, _options).await?;

    let organized_document = Document::from_basis_transformations(Arc::clone(&meta_context));

    organized_document
}

#[allow(dead_code)]
pub async fn organize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In organize_file");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    organize_text(Arc::clone(&provider), text, _options).await
}

#[allow(dead_code)]
pub async fn organize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
) -> Result<Document, Errors> {
    log::trace!("In organize_file_to_document");
    log::debug!("file path: {}", path);

    let meta_context = organize_file(Arc::clone(&provider), path, _options).await?;

    let organized_document = Document::from_basis_transformations(Arc::clone(&meta_context));

    organized_document
}

#[allow(dead_code)]
pub async fn organize_file_to_string<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In organize_file_to_string");
    log::debug!("file path: {}", path);

    let document = organize_file_to_document(Arc::clone(&provider), path, _options).await?;

    Ok(document.to_string(document_format))
}
