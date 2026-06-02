use std::sync::{Arc, RwLock};
use std::collections::HashSet;

use crate::document::{Document, DocumentType, DocumentRole};
use crate::document_format::DocumentFormat;
use crate::meta_context::MetaContext;
use crate::normalization::{normalize, normalize_text};
use crate::package::Package;
use crate::context::Context;
use crate::prelude::*;
use crate::provider::Provider;
use crate::normalization;

pub async fn translate<P: Provider>(
    provider: Arc<P>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate");

    let meta_context: Arc<RwLock<MetaContext>> = normalization::normalize(
        Arc::clone(&provider),
        source,
        options,
        execution_context.clone(),
    ).await?;

    match target.document_type {
        DocumentType::Html => {
            unimplemented!()
        }
        DocumentType::Json => {
            translate_json(
                Arc::clone(&provider),
                Arc::clone(&meta_context),
                target,
                options,
            )
            .await?;
        }
        DocumentType::PlainText => {
            unimplemented!()
        }
        DocumentType::JavaScript => {
            unimplemented!()
        }
        DocumentType::Xml => {
            unimplemented!()
        }
    }

    Ok(meta_context)
}


















pub async fn translate_json<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    document: Document,
    options: &Options
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_json");

    // -----------------------------------------------------------------------------------------------------
    let (contexts, graph_root) = document.get_contexts(Arc::clone(&meta_context))?;

    let mut seen = HashSet::new();
    let unique_contexts: Vec<Arc<Context>> = contexts
        .into_values()
        .filter(|context| seen.insert(context.id.clone()))
        .collect();

    for context in unique_contexts {


        log::debug!("lineage: {}", context.lineage.to_string());

        let document_node = read_lock!(context.document_node);

        log::debug!("document_node: {:?}", document_node.to_string());

    }

    // -----------------------------------------------------------------------------------------------------




    let normalized_json = Document::from_normalized_graph(
        Arc::clone(&meta_context),
        &DocumentFormat {
            format_type: DocumentType::Json,
            encoding: Some(String::from("UTF-8")),
            indent: None,
            line_ending: None,
            headers: None,
            wrap_text: None,
            exclude_nulls: None,
            custom_delimiter: None,
        }
    )?;



    // -----------------------------------------------------------------------------------------------------
    let (contexts, graph_root) = normalized_json.get_contexts(Arc::clone(&meta_context))?;

    let mut seen = HashSet::new();
    let unique_contexts: Vec<Arc<Context>> = contexts
        .into_values()
        .filter(|context| seen.insert(context.id.clone()))
        .collect();

    for context in unique_contexts {


        log::debug!("lineage: {}", context.lineage.to_string());

        let document_node = read_lock!(context.document_node);

        log::debug!("document_node: {:?}", document_node.to_string());

    }

    // -----------------------------------------------------------------------------------------------------


    unimplemented!();
}






























pub async fn translate_text_to_document<P: Provider>(
    provider: Arc<P>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In translate_text_to_document");

    let meta_context = translate_text_to_meta_context(
        Arc::clone(&provider),
        source,
        target,
        options,
        execution_context,
    ).await?;

    Document::from_translated_graph(Arc::clone(&meta_context), document_format)
}

pub async fn translate_text_to_meta_context<P: Provider>(
    provider: Arc<P>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In translate_text_to_meta_context");

    let source_document = Document::from_string(source.0, options, source.1)?;

    let target_document = {
        match target.1.role {
            DocumentRole::Instance => {
                Document::from_string(target.0, options, target.1)?
            },
            DocumentRole::Schema => {
                Document::from_schema_string(
                    Arc::clone(&provider),
                    target.0,
                    options,
                    target.1
                ).await?
            }
        }
    };

    let meta_context = translate(
        Arc::clone(&provider),
        source_document,
        target_document,
        options,
        execution_context.clone(),
    ).await?;

    Ok(meta_context)
}

pub async fn translate_text_to_package<P: Provider>(
    provider: Arc<P>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::trace!("In translate_text_to_package");

    let translated_document = translate_text_to_document(
        Arc::clone(&provider),
        source,
        target,
        options,
        document_format,
        execution_context,
    ).await?;

    Ok(Package {
        document: translated_document,
        mutations: Vec::new(),
    })
}
