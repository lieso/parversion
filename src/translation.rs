use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::normalization::normalize_document_to_meta_context;
use crate::organization::organize_text;
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::schema::Schema;
use crate::node_analysis::get_translation_schema_transformations;
use crate::document_format::DocumentFormat;
use crate::package::Package;
use crate::mutations::Mutations;

#[allow(dead_code)]
pub async fn translate<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate");

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

    log::info!("Parsing JSON schema");
    let schema = Schema::from_string(json_schema)?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_translation_schema(schema.clone());
    }

    let (contexts, graph_root) = &schema.get_contexts()?;
    
    {
        let mut lock = write_lock!(meta_context);
        lock.update_translation_schema_context(contexts.clone(), graph_root.clone());
    }

    log::info!("Getting translation schema transformations");
    let schema_transformations = get_translation_schema_transformations(
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
pub async fn translate_meta_context<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_meta_context");

    translate(Arc::clone(&provider), meta_context, _options, json_schema).await
}

#[allow(dead_code)]
pub async fn translate_text_to_meta_context<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_text_to_meta_context");

    let meta_context = organize_text(
        Arc::clone(&provider),
        text,
        _options
    ).await?;

    translate_meta_context(
        Arc::clone(&provider),
        meta_context,
        _options,
        json_schema
    ).await
}

#[allow(dead_code)]
pub async fn translate_text_to_package<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Package, Errors> {
    log::trace!("In translate_text_to_package");

    let meta_context = translate_text_to_meta_context(
        Arc::clone(&provider),
        text,
        _options,
        json_schema
    ).await?;

    let translated_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: translated_document,
        mutations: Mutations::default(),
    })
}

#[allow(dead_code)]
pub async fn translate_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    json_schema: &str,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In translate_text");

    let document = translate_text_to_package(
        provider,
        text,
        _options,
        json_schema
    ).await?;

    Ok(document.to_string(document_format))
}

#[allow(dead_code)]
pub async fn translate_document_to_meta_context<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_document_to_meta_context");

    let meta_context = normalize_document_to_meta_context(Arc::clone(&provider), document, _options).await?;

    translate_meta_context(Arc::clone(&provider), meta_context, _options, json_schema).await
}

#[allow(dead_code)]
pub async fn translate_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Document, Errors> {
    log::trace!("In translate_document");

    let meta_context = translate_document_to_meta_context(
        provider.clone(),
        document,
        _options,
        json_schema
    ).await?;

    let translated_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    );

    translated_document
}

#[allow(dead_code)]
pub async fn translate_document_to_text<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Option<Options>,
    json_schema: &str,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In translate_document_to_text");

    let document = translate_document(
        provider,
        document,
        _options,
        json_schema
    ).await?;

    Ok(document.to_string(document_format))
}

#[allow(dead_code)]
pub async fn translate_file_to_meta_context<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_file_to_meta_context");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    translate_text_to_meta_context(Arc::clone(&provider), text, _options, json_schema).await
}

#[allow(dead_code)]
pub async fn translate_file_to_package<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    json_schema: &str,
) -> Result<Package, Errors> {
    log::trace!("In translate_file_to_package");

    let meta_context = translate_file_to_meta_context(Arc::clone(&provider), path, _options, json_schema).await?;

    let translated_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: translated_document,
        mutations: Mutations::default(),
    })
}

#[allow(dead_code)]
pub async fn translate_file_to_text<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    json_schema: &str,
    document_format: &Option<DocumentFormat>
) -> Result<String, Errors> {
    log::trace!("In translate_file_to_text");

    let package = translate_file_to_package(
        provider,
        path,
        _options,
        json_schema
    ).await?;

    Ok(package.to_string(document_format))
}

#[allow(dead_code)]
pub async fn translate_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    json_schema: &str,
    document_format: &Option<DocumentFormat>
) -> Result<(), Errors> {
    log::trace!("In translate_file");
    log::debug!("file path: {}", path);

    let text = translate_file_to_text(
        Arc::clone(&provider),
        path,
        _options,
        json_schema,
        document_format
    ).await?;
    let new_path = append_to_filename(path, "_translated")?;

    write_text_to_file(&new_path, &text).map_err(|err| {
        log::error!("Failed to write translated text to file: {:?}", err);
        Errors::FileOutputError
    })
}

#[allow(dead_code)]
pub async fn translate_url_to_meta_context<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    json_schema: &str
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_url_to_meta_context");
    log::debug!("url: {}", url);

    let text = fetch_url_as_text(url).await?;

    translate_text_to_meta_context(Arc::clone(&provider), text, _options, json_schema).await
}

#[allow(dead_code)]
pub async fn translate_url_to_package<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    json_schema: &str
) -> Result<Package, Errors> {
    log::trace!("In translate_url_to_package");
    log::debug!("url: {}", url);

    let meta_context = translate_url_to_meta_context(Arc::clone(&provider), url, _options, json_schema).await?;

    let translated_document = Document::from_schema_transformations(
        Arc::clone(&meta_context),
        DocumentVersion::OrganizedDocument
    )?;

    Ok(Package {
        document: translated_document,
        mutations: Mutations::default(),
    })
}
