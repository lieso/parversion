use std::sync::{Arc, RwLock};
use serde_json::{json, Value, Map};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::graph_node::{Graph, GraphNode};
use crate::json_node::JsonNode;
use crate::context::Context;
use crate::document::{DocumentMetadata, DocumentType};
use crate::document_node::{DocumentNode, DocumentNodeData};
use crate::data_node::DataNode;
use crate::meta_context::MetaContext;
use crate::translation_node::TranslationNode;
use crate::translation_network::TranslationNetwork;

pub struct Json {}

impl Json {
    pub fn to_meta_context(
        metadata: &DocumentMetadata,
        data: String
    ) -> Result<MetaContext, Errors> {
        log::trace!("In to_meta_context");

        let document_root = Self::get_document_node(data)?;
        let document_root = Arc::new(RwLock::new(document_root.clone()));

        let mut contexts: HashMap<ContextID, Arc<Context>> = HashMap::new();
        let mut contexts_lookup: HashMap<ID, Arc<Context>> = HashMap::new();

        fn recurse(
            document_node: Arc<RwLock<DocumentNode>>,
            parent_lineage: &Lineage,
            contexts: &mut HashMap<ContextID, Arc<Context>>,
            contexts_lookup: &mut HashMap<ID, Arc<Context>>,
            parents: Vec<Arc<RwLock<GraphNode>>>,
        ) -> Arc<RwLock<GraphNode>> {
            let (hash, lineage, fields, description, network_name) = {
                let lock = read_lock!(document_node);
                let hash = lock.get_hash();
                let lineage = parent_lineage.with_hash(hash.clone());
                (hash, lineage, lock.get_fields(), lock.get_description(), lock.get_name())
            };

            let data_node = Arc::new(DataNode::new(
                hash,
                lineage.clone(),
                fields,
                description,
            ));

            let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
                Arc::clone(&data_node),
                parents.clone(),
            )));

            let context = Arc::new(Context {
                id: ID::new(),
                acyclic_lineage: data_node.lineage.acyclic(),
                lineage: data_node.lineage.clone(),
                document_node: Arc::clone(&document_node),
                graph_node: Arc::clone(&graph_node),
                data_node: Arc::clone(&data_node),
                network_name
            });

            contexts.insert(context.id.clone(), Arc::clone(&context));
            contexts_lookup.insert(data_node.id.clone(), Arc::clone(&context));
            contexts_lookup.insert(read_lock!(document_node).id.clone(), Arc::clone(&context));
            contexts_lookup.insert(read_lock!(graph_node).id.clone(), Arc::clone(&context));

            {
                let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
                    .get_children()
                    .into_iter()
                    .map(|child| {
                        recurse(
                            Arc::new(RwLock::new(child)),
                            &data_node.lineage,
                            contexts,
                            contexts_lookup,
                            vec![Arc::clone(&graph_node)],
                        )
                    })
                    .collect();

                let mut write_lock = graph_node.write().unwrap();

                let child_hashes: Vec<Hash> = children
                    .iter()
                    .map(|child| read_lock!(child).hash.clone())
                    .collect();

                let mut subgraph_hash = Hash::from_items(child_hashes.clone());
                let subgraph_hash = subgraph_hash
                    .sort()
                    .push(write_lock.hash.clone())
                    .finalize();

                write_lock.subgraph_hash = subgraph_hash.clone();
                write_lock.children.extend(children);
            }

            graph_node
        }

        let origin_hash = Hash::from_str(&metadata.origin.clone().unwrap_or_default());
        let initial_lineage = Lineage::new().with_hash(origin_hash);

        let graph_root = recurse(
            Arc::clone(&document_root),
            &initial_lineage,
            &mut contexts,
            &mut contexts_lookup,
            Vec::new(),
        );

        let acyclic_subgraph_hash = {
            let lock = read_lock!(graph_root);
            lock.acyclic_subgraph_hash()
        };

        Ok(MetaContext {
            contexts,
            graph_root,
            contexts_lookup,
            document_type: DocumentType::Json,
            acyclic_subgraph_hash,
        })
    }

    pub fn from_meta_context(
        meta_context: &MetaContext,
        render_ids: Option<&HashSet<GraphNodeID>>,
    ) -> Result<String, Errors> {

        let graph_root = meta_context.graph_root.clone();

        let mut result: Value = Value::Object(Map::new());

        fn recurse(
            meta_context: &MetaContext,
            render_ids: Option<&HashSet<GraphNodeID>>,
            graph_node: Graph,
            network_name: &str,
            result: &mut Value
        ) {
            let should_render = if let Some(render_ids) = render_ids {
                render_ids.contains(&read_lock!(graph_node).id)
            } else {
                true
            };

            let context = meta_context.contexts_lookup.get(&read_lock!(graph_node).id).unwrap();

            if should_render {
                if !result.is_object() {
                    *result = Value::Object(Map::new());
                }
                let data_node = &context.data_node;
                let json_nodes: Vec<JsonNode> = data_node.to_json_nodes();
                for json_node in json_nodes {
                    let json = json_node.json;
                    let value = json!(json.value.trim().to_string());
                    if let Value::Object(ref mut map) = result {
                        map.insert(json.key, value);
                    }
                }
            }

            for child in &read_lock!(graph_node).children {
                let child_context = meta_context.contexts_lookup.get(&read_lock!(child).id).unwrap();
                let next_network_name = if child_context.network_name.is_empty() {
                    network_name
                } else {
                    &child_context.network_name
                };

                let should_render_child = if let Some(render_ids) = render_ids {
                    render_ids.contains(&read_lock!(child).id)
                } else {
                    true
                };

                if should_render_child {
                    if child_context.network_name.is_empty() {
                        let mut inner_result: Value = Value::Object(Map::new());

                        recurse(
                            meta_context,
                            render_ids.clone(),
                            Arc::clone(&child),
                            next_network_name,
                            &mut inner_result
                        );

                        let inner_result_value = inner_result.clone();

                        if result.is_object() && result.as_object().unwrap().is_empty() {
                            *result = inner_result;
                        } else if result.is_array() {
                            if let Value::Array(ref mut arr) = result {
                                arr.push(inner_result_value.clone());
                            }
                        } else {
                            *result = json!(vec![
                                result.clone(),
                                inner_result_value.clone()
                            ]);
                        }

                    } else {
                        let mut inner_result: Value = Value::Object(Map::new());

                        recurse(
                            meta_context,
                            render_ids.clone(),
                            Arc::clone(&child),
                            next_network_name,
                            &mut inner_result
                        );

                        let inner_result_value = inner_result.clone();

                        if !result.is_object() {
                            *result = Value::Object(Map::new());
                        }

                        if let Value::Object(ref mut map) = result {
                            if let Some(existing_object) = map.get_mut(next_network_name) {
                                if let Value::Array(ref mut arr) = existing_object {
                                    arr.push(inner_result_value.clone());
                                } else {
                                    *existing_object = json!(vec![
                                        existing_object.clone(),
                                        inner_result_value.clone()
                                    ]);
                                }
                            } else {
                                map.insert(next_network_name.to_string(), inner_result_value);
                            }
                        }
                    }
                } else {
                    if let Value::Object(ref mut map) = result {
                        map.insert("_omitted".to_string(), json!(true));
                    }
                    recurse(
                        meta_context,
                        render_ids.clone(),
                        Arc::clone(&child),
                        next_network_name,
                        result,
                    );
                }
            }
        }

        recurse(
            meta_context,
            render_ids.clone(),
            Arc::clone(&graph_root),
            "test",
            &mut result,
        );

        let data = serde_json::to_string_pretty(&result).expect("Could not make a JSON string");

        Ok(data)
    }

    pub fn from_translation(
        translation_context: Arc<RwLock<TranslationContext>>
    ) -> Result<String, Errors> {
        log::trace!("In from_translation");

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

        Ok(serde_json::to_string_pretty(&result).expect("Could not make a JSON string"))
    }

    pub fn from_normalized_graph(
        normalization_context: Arc<RwLock<NormalizationContext>>,
    ) -> Result<String, Errors> {
        log::trace!("In from_normalized_graph_json");

        let graph_root = read_lock!(normalization_context).normal_graph_root.clone().unwrap();

        let mut result: Map<String, Value> = Map::new();

        fn recurse(
            normalization_context: Arc<RwLock<NormalizationContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>,
        ) {
            let contexts = {
                let lock = read_lock!(normalization_context);
                lock.normal_contexts.clone().unwrap()
            };

            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let network_name = &context.network_name;
            let network_description = &context.network_description;
            let data_node = &context.data_node;
            let json_nodes: Vec<JsonNode> = data_node.to_json_nodes();

for json_node in json_nodes {
                let json = json_node.json;
                let value = json!(json.value.trim().to_string());
                result.insert(json.key, value);
            }

            for child in &read_lock!(graph_node).children {
                let child_context = contexts.get(&read_lock!(child).id).unwrap();

                if let Some(child_network_name) = &child_context.network_name {
                    let mut inner_result: Map<String, Value> = Map::new();

                    recurse(
                        Arc::clone(&normalization_context),
                        Arc::clone(&child),
                        &mut inner_result
                    );

                    let inner_result_value = Value::Object(inner_result.clone());

                    if let Some(existing_object) = result.get_mut(child_network_name) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }
                    } else {
                        result.insert(child_network_name.clone(), inner_result_value);
                    }

                } else {
                    recurse(
                        Arc::clone(&normalization_context),
                        Arc::clone(&child),
                        result
                    );
                }
            }
        }

        recurse(
            Arc::clone(&normalization_context),
            Arc::clone(&graph_root),
            &mut result,
        );

        let data = serde_json::to_string_pretty(&result).expect("Could not make a JSON string");

        Ok(data)
    }

    fn get_document_node(data: String) -> Result<DocumentNode, Errors> {
        let value: Value = serde_json::from_str(&data)
            .map_err(|e| {
                Errors::JsonParseError(e.to_string())
            })?;

        match value {
            serde_json::Value::Object(map) => Ok(DocumentNode::new(DocumentNodeData::Json(map))),
            _ => Err(Errors::JsonParseError("JSON root must be an object".to_string())),
        }
    }
}
