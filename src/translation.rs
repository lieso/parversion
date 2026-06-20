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
use crate::network_analysis::{get_translation_networks};
use crate::graph_node::Graph;
use crate::translation_node::TranslationNode;
use crate::translation_network::TranslationNetwork;
use crate::data_node::DataNode;

pub async fn translate<P: Provider>(
    provider: Arc<P>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In translate");

    let translation_context = init_translation_context(
        Arc::clone(&provider),
        source,
        target,
        options,
        execution_context.clone(),
    ).await?;

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

    let stage = execution_context.enter_stage("Translating networks");

    let translation_networks =
        get_translation_networks(
            Arc::clone(&provider),
            Arc::clone(&translation_context),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(translation_context);
        lock.update_translation_networks(translation_networks);
    }

    stage.finish();

    Ok(translation_context)
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

    log::debug!("normalized_document: {}", normalized_document.to_string());

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

    let translation_context = translate_text(
        Arc::clone(&provider),
        source,
        target,
        options,
        execution_context,
    ).await?;

    unimplemented!()
}

pub async fn translate_text<P: Provider>(
    provider: Arc<P>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In translate_text");

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

    let translation_context = translate(
        Arc::clone(&provider),
        source_document,
        target_document,
        options,
        execution_context.clone(),
    ).await?;

    Ok(translation_context)
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

async fn init_translation_context<P: Provider>(
    provider: Arc<P>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In init_translation_context");

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

    Ok(translation_context)
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
            let data_node = &current_context.data_node;

            let translated: Vec<DataNode> = translation_node
                .transformations
                .iter()
                .map(|transformation| transformation.transform(data_node.clone()).expect("Could not transform"))
                .collect();

            for node in translated {
                for (key, value) in node.fields {
                    let json_value = json!(value.trim().to_string());
                    if let Value::Object(ref mut map) = result {
                        map.insert(key.clone(), json_value);
                    }
                }
            }
        }

        let translation_network: Option<Arc<TranslationNetwork>> = {
            let lock = read_lock!(translation_context);
            lock.translation_networks
                .as_ref()
                .unwrap()
                .values()
                .cloned()
                .find(|item| item.source_lineage == current_context.lineage)
        };

        if let Some(translation_network) = translation_network {
            let transformation = &translation_network.transformation;
            
            if transformation.cardinality == "array" {
                for child in &read_lock!(graph_node).children {
                    let mut inner_result: Value = Value::Object(Map::new());

                    recurse(
                        Arc::clone(&translation_context),
                        Arc::clone(&child),
                        &mut inner_result
                    );

                    if let Value::Object(ref mut map) = result {
                        match map.entry(transformation.image.clone()) {
                            serde_json::map::Entry::Vacant(entry) => {
                                entry.insert(json!(vec![inner_result]));
                            }
                            serde_json::map::Entry::Occupied(mut entry) => {
                                let existing = entry.get_mut();
                                if let Value::Array(ref mut arr) = existing {
                                    arr.push(inner_result)
                                }
                            }
                        }
                    }
                }
            } else {
                let mut inner_result: Value = Value::Object(Map::new());

                for child in &read_lock!(graph_node).children {
                    recurse(
                        Arc::clone(&translation_context),
                        Arc::clone(&child),
                        &mut inner_result
                    );
                }

                if let Value::Object(ref mut map) = result {
                    map.insert(transformation.image.clone(), inner_result);
                }
            }

        } else {
            for child in &read_lock!(graph_node).children {
                recurse(
                    Arc::clone(&translation_context),
                    Arc::clone(&child),
                    result
                );
            }
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
