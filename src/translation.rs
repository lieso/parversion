use std::sync::{Arc, RwLock};
use serde_json::{json, Value, Map};

use crate::document::{Document, DocumentType, DocumentRole};
use crate::document_format::DocumentFormat;
use crate::normalization_context::NormalizationContext;
use crate::translation_context::TranslationContext;
use crate::normalization::normalize;
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::normalization;
use crate::node_analysis::{get_translation_nodes};
use crate::graph_node::Graph;
use crate::translation_node::TranslationNode;
use crate::data_node::DataNode;

pub async fn translate<P: Provider>(
    provider: Arc<P>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    log::trace!("In translate");

    let normalization_context: Arc<RwLock<NormalizationContext>> = normalization::normalize(
        Arc::clone(&provider),
        source,
        options,
        execution_context.clone(),
    ).await?;

    let translation_context = Arc::new(RwLock::new(TranslationContext::new()));

    match target.document_type {
        DocumentType::Html => {
            unimplemented!()
        }
        DocumentType::Json => {
            translate_json(
                Arc::clone(&provider),
                Arc::clone(&normalization_context),
                Arc::clone(&translation_context),
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



    let stage = execution_context.enter_stage("Translating nodes");

let translation_nodes = 
        get_translation_nodes(
            Arc::clone(&provider),
            Arc::clone(&translation_context),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(translation_context);
        lock.update_translation_nodes(translation_nodes);
    }

    stage.finish();



    do_something(Arc::clone(&translation_context))?;



    unimplemented!();


    Ok(normalization_context)
}


















pub async fn translate_json<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    translation_context: Arc<RwLock<TranslationContext>>,
    document: Document,
    options: &Options
) -> Result<(), Errors> {
    log::trace!("In translate_json");

    let translation_meta_context = document.to_meta_context()?;

    let normalized_document = Document::from_normalized_graph(
        Arc::clone(&normalization_context),
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

    let normalized_meta_context = normalized_document.to_meta_context()?;

    {
        let mut lock = write_lock!(translation_context);
        lock.update_meta_contexts(normalized_meta_context, translation_meta_context);
    }

    Ok(())
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

    let normalization_context = translate_text_to_meta_context(
        Arc::clone(&provider),
        source,
        target,
        options,
        execution_context,
    ).await?;

    unimplemented!()
}

pub async fn translate_text_to_meta_context<P: Provider>(
    provider: Arc<P>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
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

    let normalization_context = translate(
        Arc::clone(&provider),
        source_document,
        target_document,
        options,
        execution_context.clone(),
    ).await?;

    Ok(normalization_context)
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






fn do_something(translation_context: Arc<RwLock<TranslationContext>>) -> Result<(), Errors> {



    let graph_root: Graph = {
        let lock = read_lock!(translation_context);
        let meta_context = lock.input_meta_context.as_ref().unwrap();
        meta_context.graph_root.clone()
    };


    let mut result: Value = Value::Object(Map::new());

    fn recurse(
        translation_context: Arc<RwLock<TranslationContext>>,
        graph_node: Graph,
        result: &mut Value
    ) {
        let current_context = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.input_meta_context.as_ref().unwrap();
            meta_context.contexts_lookup.get(&read_lock!(graph_node).id).unwrap().clone()
        };

        let translation_node: Option<Arc<TranslationNode>> = {
            let lock = read_lock!(translation_context);
            lock.translation_nodes
                .as_ref()
                .unwrap()
                .values()
                .cloned()
                .find(|item| item.source_lineage == current_context.lineage)
        };

        if let Some(translation_node) = translation_node {

            let any_target_context = {
                let lock = read_lock!(translation_context);
                let meta_context = lock.target_meta_context.as_ref().unwrap();
                meta_context.contexts
                    .values()
                    .find(|item| item.lineage == translation_node.target_lineage)
                    .cloned()
                    .unwrap()
            };

            let mut network_name: String = any_target_context.network_name.clone();
            let mut graph_node = any_target_context.graph_node.clone();

            while network_name.is_empty() {
                let parent = {
                    let lock = read_lock!(graph_node);
                    lock.parents.first().cloned()
                };

                if let Some(parent) = parent {
                    let parent_context = {
                        let lock = read_lock!(translation_context);
                        let meta_context = lock.target_meta_context.as_ref().unwrap();
                        meta_context.contexts_lookup.get(&read_lock!(parent).id).unwrap().clone()
                    };

                    network_name = parent_context.network_name.clone();
                    graph_node = Arc::clone(&parent_context.graph_node);
                } else {
                    break;
                }
            }

            log::debug!("network_name: {}", network_name);

        }
        
        for child in &read_lock!(graph_node).children {
            recurse(
                Arc::clone(&translation_context),
                Arc::clone(&child),
                result,
            );
        }

    }

    recurse(
        Arc::clone(&translation_context),
        Arc::clone(&graph_root),
        &mut result
    );


    let data = serde_json::to_string_pretty(&result).expect("Could not make a JSON string");
    

    log::debug!("data: {}", data);


    unimplemented!()
}
