use std::sync::Arc;

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organization::{organize};
use crate::model::{Model};
use crate::provider::{Provider};
use crate::traverse::{build_document_from_nodeset};

pub async fn normalize<P: Provider>(
    provider: Arc<P>,
    nodeset: NodeSet,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize");
    
    unimplemented!()

    //let basis_graph = nodeset.build_basis_graph()?;

    //let target_model = Model::get_normal_model(&basis_graph).unwrap();

    //nodeset.transmute(&target_model.json_schema).await
}

pub async fn normalize_nodeset<P: Provider>(
    provider: Arc<P>,
    nodeset: NodeSet,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize_nodeset");

    normalize(Arc::clone(&provider), nodeset, options).await
}

pub async fn normalize_text_to_nodeset<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize_text_to_nodeset");

    let document = Document::from_string(text, options)?;
    let nodeset = organize(Arc::clone(&provider), document, options).await?;

    normalize_nodeset(Arc::clone(&provider), nodeset, options).await
}

pub async fn normalize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let nodeset = normalize_text_to_nodeset(Arc::clone(&provider), text, options).await?;

    build_document_from_nodeset(
        provider,
        nodeset,
        document_format,
    )
}

pub async fn normalize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let document = normalize_text_to_document(
        Arc::clone(&provider),
        text,
        options,
        document_format
    ).await?;

    Ok(document.to_string())
}

pub async fn normalize_document_to_nodeset<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize_document_to_nodeset");

    let nodeset = organize(Arc::clone(&provider), document, options).await?;

    normalize_nodeset(Arc::clone(&provider), nodeset, options).await
}

pub async fn normalize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let nodeset = normalize_document_to_nodeset(Arc::clone(&provider), document, options).await?;

    build_document_from_nodeset(
        provider,
        nodeset,
        document_format,
    )
}

pub async fn normalize_document_to_text<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(Arc::clone(&provider), document, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file_to_nodeset<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize_file_to_nodeset");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path)?;

    normalize_text_to_nodeset(Arc::clone(&provider), text, options).await
}

pub async fn normalize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let nodeset = normalize_file_to_nodeset(Arc::clone(&provider), path, options).await?;

    build_document_from_nodeset(
        provider,
        nodeset,
        document_format,
    )
}

pub async fn normalize_file_to_text<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(Arc::clone(&provider), path, options, document_format).await?;

    Ok(document.to_string())
}

pub async fn normalize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<(), Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = normalize_file_to_text(Arc::clone(&provider), path, options, document_format).await?;
    let new_path = append_to_filename(path, "_normalized")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}

pub async fn normalize_url_to_nodeset<P: Provider>(
    provider: Arc<P>,
    url: &str,
    options: &Option<Options>,
) -> Result<NodeSet, Errors> {
    log::trace!("In normalize_url_to_nodeset");
    log::debug!("URL: {}", url);

    let text = fetch_url_as_text(url).await?;

    normalize_text_to_nodeset(Arc::clone(&provider), text, options).await
}

pub async fn normalize_url_to_document<P: Provider>(
    provider: Arc<P>,
    url: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_url_to_document");
    log::debug!("URL: {}", url);

    let nodeset = normalize_url_to_nodeset(Arc::clone(&provider), url, options).await?;

    build_document_from_nodeset(
        provider,
        nodeset,
        document_format,
    )
}

pub async fn normalize_url_to_text<P: Provider>(
    provider: Arc<P>,
    url: &str,
    options: &Option<Options>,
    document_format: &Option<DocumentFormat>,
) -> Result<String, Errors> {
    log::trace!("In normalize_url_to_text");
    log::debug!("URL: {}", url);

    let document = normalize_url_to_document(Arc::clone(&provider), url, options, document_format).await?;

    Ok(document.to_string())
}
