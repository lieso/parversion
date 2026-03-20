use serde_json::{json, Value, Map};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{GraphNode, GraphNodeID};
use crate::json_node::JsonNode;
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::provider::Provider;

pub type ContextID = ID;

#[derive(Clone, Debug)]
pub struct Context {
    pub id: ContextID,
    pub lineage: Lineage,
    pub document_node: Arc<RwLock<DocumentNode>>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
}

impl Context {
    pub fn generate_json_snippet(
        &self,
        meta_context: Arc<RwLock<MetaContext>>
    ) -> Result<String, Errors> {



        let mut result: Map<String, Value> = Map::new();


        fn recurse(
            meta_context: Arc<RwLock<MetaContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>
        ) {



            let contexts = {
                let lock = read_lock!(meta_context);
                lock.contexts.clone().unwrap()
            };





            let mut subgraph_counter: HashMap<String, u32> = HashMap::new();
            for child in &read_lock!(graph_node).children {
                let subgraph_hash: String = read_lock!(child).subgraph_hash.clone().to_string().unwrap();

                let count = *subgraph_counter.entry(subgraph_hash.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);

                if count <= 3 {
                    let mut inner_result: Map<String, Value> = Map::new();

                    recurse(
                        Arc::clone(&meta_context),
                        Arc::clone(&child),
                        &mut inner_result,
                    );

                    let inner_result_value = Value::Object(inner_result.clone());

                    if let Some(existing_object) = result.get_mut(&subgraph_hash) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }
                    } else {
                        if inner_result.len() > 0 {
                            result.insert(subgraph_hash.to_string(), inner_result_value);
                        }
                    }
                }
            }


            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let data_node = &context.data_node;
            let basis_node = {
                let lock = read_lock!(meta_context);
                lock.get_basis_node_by_lineage(&context.lineage).expect("Could not get basis node by lineage").unwrap()
            };
            let json_nodes: Vec<JsonNode> = basis_node
                .transformations
                .clone()
                .into_iter()
                .map(|transformation| {
                    transformation
                        .transform(Arc::clone(&data_node))
                        .expect("Could not transform data node field")
                })
                .collect();

            let mut json_data: Map<String, Value> = Map::new();

            for json_node in json_nodes.into_iter() {
                let json = json_node.json;
                let value = json!(json.value.trim().to_string());
                json_data.insert(json.key, value);
            }

            if json_data.len() > 0 {
                result.insert("json".to_string(), Value::Object(json_data));
            }



        }

        recurse(
            Arc::clone(&meta_context),
            Arc::clone(&self.graph_node),
            &mut result,
        );

        Ok(serde_json::to_string_pretty(&result).expect("Could not make a JSON string"))
    }
    
    pub fn generate_snippet(&self, meta_context: Arc<RwLock<MetaContext>>) -> String {
        let mut neighbour_ids = HashSet::new();
        let graph_node = self.graph_node.clone();

        Self::traverse_for_neighbours(Arc::clone(&graph_node), &mut neighbour_ids);

        let mut snippet = String::new();
        let lock = read_lock!(meta_context);
        let graph_root = lock.graph_root.clone().unwrap();

        Self::traverse_for_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&graph_root),
            &neighbour_ids,
            &read_lock!(graph_node).id,
            &mut snippet,
        );

        snippet
    }

    fn traverse_for_snippet(
        meta_context: Arc<RwLock<MetaContext>>,
        current_node: Arc<RwLock<GraphNode>>,
        neighbour_ids: &HashSet<GraphNodeID>,
        target_id: &GraphNodeID,
        snippet: &mut String,
    ) {
        let meta_context_lock = read_lock!(meta_context);
        let lock = read_lock!(current_node);
        let current_id = lock.id.clone();
        let contexts = meta_context_lock.contexts.as_ref().unwrap();
        let current_context = contexts.get(&current_id).unwrap();
        let document_node = current_context.document_node.clone();

        let should_render = if current_id == *target_id {
            let (mut a, _b) = read_lock!(document_node).to_string_components();

            a = Self::mark_text(&a);
            snippet.push_str(&a);

            true
        } else if neighbour_ids.contains(&current_id) {
            let (a, _b) = read_lock!(document_node).to_string_components();

            snippet.push_str(&a);

            true
        } else {
            false
        };

        for child in &lock.children {
            Self::traverse_for_snippet(
                Arc::clone(&meta_context),
                Arc::clone(child),
                neighbour_ids,
                target_id,
                snippet,
            );
        }

        if should_render {
            let (_, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(b.as_deref().unwrap_or(""));
        }
    }

    fn traverse_for_neighbours(
        start_node: Arc<RwLock<GraphNode>>,
        visited: &mut HashSet<GraphNodeID>,
    ) {
        let mut queue: VecDeque<Arc<RwLock<GraphNode>>> = VecDeque::new();
        queue.push_back(Arc::clone(&start_node));

        while let Some(node) = queue.pop_front() {
            let lock = read_lock!(node);
            let graph_node_id = lock.id.clone();

            if visited.contains(&graph_node_id) {
                continue;
            }

            visited.insert(graph_node_id.clone());

            if visited.len() > 20 {
                return;
            }

            for child in lock.children.iter() {
                queue.push_back(Arc::clone(child));
            }

            for parent in lock.parents.iter() {
                queue.push_back(Arc::clone(parent));
            }
        }
    }

    fn mark_text(text: &str) -> String {
        let marker_prefix = "<!-- Target node: Start -->";
        let marker_suffix = "<!-- Target node: End -->";

        format!("{}{}{}", marker_prefix, text, marker_suffix)
    }
}
