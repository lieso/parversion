use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::organization::{organize};
use crate::provider::{Provider};
use crate::meta_context::MetaContext;
use crate::node_analysis::{get_normal_schema_transformations};
use crate::document_format::DocumentFormat;
use crate::package::Package;
use crate::mutations::Mutations;

#[allow(dead_code)]
pub async fn normalize<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize");

    log::info!("Generating organized document");
    let document = Document::from_basis_transformations(Arc::clone(&meta_context))?;

    {
        let mut lock = write_lock!(meta_context);
        lock.add_document_version(DocumentVersion::OrganizedDocument, document.clone());
    }

    {
        log::info!("Getting schema context");
        let (contexts, graph_root) = &document.schema.unwrap().get_contexts()?;
        let mut lock = write_lock!(meta_context);
        lock.update_schema_context(contexts.clone(), graph_root.clone());
    }

    log::info!("Getting normal schema transformations");
    let schema_transformations = get_normal_schema_transformations(
        Arc::clone(&provider),
        Arc::clone(&meta_context)
    ).await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_schema_transformations(schema_transformations);
    }

    {
        let normalized = Document::from_schema_transformations(
            Arc::clone(&meta_context),
            DocumentVersion::OrganizedDocument
        )?;
        let result = format!("{}", normalized.to_string(&None));
        log::debug!("\n\n\
        =======================================================\n\
        =============   NORMALIZED DOCUMENT START   =================\n\
        =======================================================\n\
        {}
        =======================================================\n\
        =============    NORMALIZED DOCUMENT END    =================\n\
        =======================================================\n\n", result);
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
pub async fn normalize_text_to_package<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
) -> Result<Package, Errors> {
    log::trace!("In normalize_text_to_package");

    let meta_context = normalize_text_to_meta_context(Arc::clone(&provider), text, _options).await?;

    let normalized_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: normalized_document,
        mutations: Mutations::default(),
    })
}

#[allow(dead_code)]
pub async fn normalize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In normalize_text");

    let package = normalize_text_to_package(
        Arc::clone(&provider),
        text,
        _options,
    ).await?;

    Ok(package.to_string(document_format))
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
) -> Result<Document, Errors> {
    log::trace!("In normalize_document");

    let meta_context = normalize_document_to_meta_context(Arc::clone(&provider), document, _options).await?;

    let normalized_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    );

    normalized_document
}

#[allow(dead_code)]
pub async fn normalize_document_to_text<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_text");

    let document = normalize_document(Arc::clone(&provider), document, _options).await?;

    Ok(document.to_string(document_format))
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
pub async fn normalize_file_to_package<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
) -> Result<Package, Errors> {
    log::trace!("In normalize_file_to_package");
    log::debug!("file path: {}", path);

    let meta_context = normalize_file_to_meta_context(Arc::clone(&provider), path, _options).await?;

    let normalized_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: normalized_document,
        mutations: Mutations::default(),
    })
}

#[allow(dead_code)]
pub async fn normalize_file_to_text<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_text");
    log::debug!("file path: {}", path);

    let package = normalize_file_to_package(Arc::clone(&provider), path, _options).await?;

    Ok(package.to_string(document_format))
}

#[allow(dead_code)]
pub async fn normalize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
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
pub async fn normalize_url_to_package<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
) -> Result<Package, Errors> {
    log::trace!("In normalize_url_to_package");
    log::debug!("URL: {}", url);

    let meta_context = normalize_url_to_meta_context(Arc::clone(&provider), url, _options).await?;

    let normalized_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: normalized_document,
        mutations: Mutations::default(),
    })
}

#[allow(dead_code)]
pub async fn normalize_url_to_text<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In normalize_url_to_text");
    log::debug!("URL: {}", url);

    let package = normalize_url_to_package(Arc::clone(&provider), url, _options).await?;

    Ok(package.to_string(document_format))
}
