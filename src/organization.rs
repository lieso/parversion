use std::sync::Arc;

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::provider::Provider;
use crate::traverse::{
    TraversalWithContext,
    traverse_with_context,
    build_document_from_nodeset
};
use crate::analysis::{Analysis};

pub async fn organize<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In organize");

    //let input = NodeSetInput::from_document(Arc::clone(&provider), document).await?;
    //let analysis = NodeSet::new(Arc::clone(&provider), &input).await?;
    //Ok(analysis)

    let mut document = document;

    let profile = document.perform_analysis(provider.clone()).await?;



    let TraversalWithContext { nodeset, meta_context, contexts, .. } =   
        traverse_with_context(&profile, document)
            .expect("Could not traverse document");



    Analysis::start(Arc::clone(&provider), meta_context, contexts).await?;



    Ok(nodeset)
}

pub async fn organize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In organize_document");

    let nodeset = organize(
        Arc::clone(&provider),
        document,
        options
    ).await?;

    build_document_from_nodeset(
        provider,
        nodeset,
        document_format,
    )
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
) -> Result<NodeSet, Errors> {
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

    let nodeset = organize_text(Arc::clone(&provider), text, options).await?;

    build_document_from_nodeset(provider, nodeset, document_format)
}

pub async fn organize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
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

    let nodeset = organize_file(Arc::clone(&provider), path, options).await?;

    build_document_from_nodeset(provider, nodeset, document_format)
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
