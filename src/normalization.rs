use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::organization::{organize};
use crate::provider::{Provider};
use crate::traverse::{build_document_from_meta_context};
use crate::meta_context::MetaContext;
use crate::schema::Schema;

#[allow(dead_code)]
pub async fn normalize<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize");

    let basis_graph = {
        let lock = read_lock!(meta_context);
        lock.basis_graph.clone().unwrap()
    };


    let document = build_document_from_meta_context(
        meta_context.clone(),
        &None
    )?;

    println!("{}", document.to_string());

    unimplemented!();



    let current_schema = Arc::new(Schema::from_meta_context(Arc::clone(&meta_context))?);


    if let Some(normal_schema) = provider.get_schema_by_basis_graph(&basis_graph).await? {
        log::info!("Found a normal schema for basis graph");

        let schema_transformations = current_schema.get_schema_transformations(
            Arc::clone(&provider),
            Arc::new(normal_schema),
        ).await?;

        {
            let mut lock = write_lock!(meta_context);
            lock.update_schema_transformations(schema_transformations);
        }

    } else {
        log::info!("Did not find a normal schema for basis graph");

        let (normal_schema, schema_transformations) = current_schema.new_normal_schema(
            Arc::clone(&provider),
        ).await?;

        {
            let mut lock = write_lock!(meta_context);
            lock.update_schema_transformations(schema_transformations);
        }
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

    build_document_from_meta_context(
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

    build_document_from_meta_context(
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

    build_document_from_meta_context(
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

    build_document_from_meta_context(
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
