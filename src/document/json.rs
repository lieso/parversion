use std::sync::{Arc, RwLock};
use serde_json::{json, Value, Map};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::graph_node::GraphNode;
use crate::json_node::JsonNode;
use crate::context::Context;
use crate::document::{Document, DocumentType, DocumentMetadata};
use crate::document_node::{DocumentNode, DocumentNodeData};
use crate::data_node::DataNode;
use crate::meta_context::MetaContext;

pub struct Json {}

impl Json {
    pub fn generate_meta_context(
        metadata: &DocumentMetadata,
        data: String
    ) -> Result<MetaContext, Errors> {
        log::trace!("In generate_meta_context");

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
            let (hash, lineage, fields, description) = {
                let lock = read_lock!(document_node);
                let hash = lock.get_hash();
                let lineage = parent_lineage.with_hash(hash.clone());
                (hash, lineage, lock.get_fields(), lock.get_description())
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

        Ok(MetaContext {
            contexts,
            graph_root,
            contexts_lookup,
        })
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
                    log::debug!("child_network_name: {}", child_network_name);

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
