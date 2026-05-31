use std::sync::{Arc, RwLock};

use crate::document::{Document, DocumentType, DocumentRole};
use crate::document_format::DocumentFormat;
use crate::meta_context::MetaContext;
use crate::normalization::{normalize, normalize_text};
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::normalization;

pub async fn translate<P: Provider>(
    provider: Arc<P>,
    normalized: Arc<RwLock<MetaContext>>,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate");


    unimplemented!()
}

pub async fn translate_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    metadata: &Metadata,
    translation: String,
    translation_metadata: &Metadata,
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In translate_text_to_document");

    let meta_context = translate_text_to_meta_context(
        Arc::clone(&provider),
        text,
        metadata,
        translation,
        translation_metadata,
        options,
        execution_context,
    ).await?;

    Document::from_translated_graph(Arc::clone(&meta_context), document_format)
}

pub async fn translate_text_to_meta_context<P: Provider>(
    provider: Arc<P>,
    text: String,
    metadata: &Metadata,
    translation: String,
    translation_metadata: &Metadata,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_text_to_meta_context");

    let meta_context: Arc<RwLock<MetaContext>> = normalization::normalize_text(
        Arc::clone(&provider),
        text,
        options,
        metadata,
        execution_context.clone(),
    ).await?;

    let document: Document = {
        match translation_metadata.role {
            DocumentRole::Instance => {
                Document::from_string(translation, options, metadata)?
            },
            DocumentRole::Schema => {
                Document::from_schema_string(
                    Arc::clone(&provider),
                    translation,
                    options,
                    translation_metadata
                ).await?
            }
        }
    };

    translate(
        Arc::clone(&provider),
        Arc::clone(&meta_context),
        document,
        options,
        execution_context.clone(),
    ).await?;

    Ok(meta_context)
}

pub async fn translate_text_to_package<P: Provider>(
    provider: Arc<P>,
    text: String,
    metadata: &Metadata,
    translation: String,
    translation_metadata: &Metadata,
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::trace!("In translate_text_to_package");

    let translated_document = translate_text_to_document(
        Arc::clone(&provider),
        text,
        metadata,
        translation,
        translation_metadata,
        options,
        document_format,
        execution_context,
    ).await?;

    Ok(Package {
        document: translated_document,
        mutations: Vec::new(),
    })
}
