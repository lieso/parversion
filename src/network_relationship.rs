use std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};
use serde_json::{json, Value, Map};

use crate::prelude::*;
use crate::basis_network::{BasisNetwork, NetworkType};
use crate::provider::Provider;
use crate::graph_node::{Graph, GraphNode};
use crate::json_node::JsonNode;
use crate::document::Document;

pub struct NetworkRelationship {}

impl NetworkRelationship {
    pub async fn explore_relationships<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<RwLock<MetaContext>>,
        networks: Vec<Arc<BasisNetwork>>
    ) -> Result<(), Errors> {

        let graph_root = {
            let lock = read_lock!(meta_context);
            lock.graph_root
                .clone()
                .ok_or(Errors::GraphRootNotProvided)?
        };

        let mut all_network_jsons = String::new();

        for network in networks.iter() {
            let json_examples = Self::get_network_json(
                Arc::clone(&meta_context),
                network.clone()
            ).await?;

            if json_examples.is_empty() {
                continue;
            }

            let examples_string: String = json_examples.iter().enumerate()
                .map(|(index, json)| format!("\nExample {}:\n{}\n", index + 1, json))
                .collect();

            let network_section = format!(
                "\n{}\n\n[Network ID]\n{}\n\n[Network examples]\n{}\n",
                "=".repeat(100),
                network.id.to_string(),
                examples_string
            );

            all_network_jsons.push_str(&network_section);
        }

        //log::debug!("{}", all_network_jsons);

        let original_document = {
            let lock = read_lock!(meta_context);
            lock.get_original_document()
        };


        let user_prompt = format!(r##"
[ORIGINAL DOCUMENT]:
{}

[NETWORKS]:
{}
"##, original_document, all_network_jsons);




        log::debug!("{}", user_prompt);

        unimplemented!()
    }

    async fn get_network_json(
        meta_context: Arc<RwLock<MetaContext>>,
        network: Arc<BasisNetwork>
    ) -> Result<Vec<String>, Errors> {

        let mut network_jsons: Vec<String> = Vec::new();

        let graph_root = read_lock!(meta_context).graph_root.clone().unwrap();

        let mut queue = VecDeque::new();
        queue.push_back(graph_root);

        while let Some(current) = queue.pop_front() {
            if network_jsons.len() >= 5 {
                break;
            }

            let subgraph_hash = {
                let lock = read_lock!(current);
                lock.subgraph_hash.clone()
            };

            if subgraph_hash == network.subgraph_hash {
                let json = Self::process_network(
                    Arc::clone(&meta_context),
                    network.clone(),
                    Arc::clone(&current)
                ).await?;

                network_jsons.push(json);
            } else {
                for child in &read_lock!(current).children {
                    queue.push_back(child.clone());
                }
            }
        }

        Ok(network_jsons)
    }

    async fn process_network(
        meta_context: Arc<RwLock<MetaContext>>,
        basis_network: Arc<BasisNetwork>,
        graph_node: Arc<RwLock<GraphNode>>
    ) -> Result<String, Errors> {

        let mut result: Map<String, Value> = Map::new();

        fn recurse(
            meta_context: Arc<RwLock<MetaContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>,
        ) {
            let mut subgraph_counter: HashMap<String, u32> = HashMap::new();

            for child in &read_lock!(graph_node).children {
                let subgraph_hash: Hash = read_lock!(child).subgraph_hash.clone();

                let count = *subgraph_counter.entry(subgraph_hash.to_string().unwrap().clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);

                if count <= 3 {
                    let basis_network: Arc<BasisNetwork> = {
                        let lock = read_lock!(meta_context);
                        lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)
                            .unwrap()
                            .expect("could not find basis network")
                    };

                    match &basis_network.transformation {
                        NetworkType::Degenerate => {
                            recurse(
                                Arc::clone(&meta_context),
                                Arc::clone(&child),
                                result,
                            );
                        },
                        NetworkType::Complex(transformation) => {
                            let mut inner_result: Map<String, Value> = Map::new();

                            recurse(
                                Arc::clone(&meta_context),
                                Arc::clone(&child),
                                &mut inner_result,
                            );

                            let inner_result_value = Value::Object(inner_result.clone());

                            if let Some(existing_object) = result.get_mut(&transformation.image) {
                                if let Value::Array(ref mut arr) = existing_object {
                                    arr.push(inner_result_value.clone());
                                } else {
                                    *existing_object = json!(vec![
                                        existing_object.clone(),
                                        inner_result_value.clone()
                                    ]);
                                }
                            } else {
                                result.insert(transformation.image.clone(), inner_result_value);
                            }
                        },
                    }
                }
            }

            let contexts = {
                let lock = read_lock!(meta_context);
                lock.contexts.clone().unwrap()
            };

            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let data_node = &context.data_node;
            let basis_node = {
                let lock = read_lock!(meta_context);
                lock.get_basis_node_by_lineage(&context.lineage)
                    .expect("Could not get basis node by lineage")
                    .unwrap()
            };

            let json_nodes: Vec<JsonNode> = basis_node.transformations
                .clone()
                .into_iter()
                .map(|transformation| {
                    transformation
                        .transform(Arc::clone(&data_node))
                        .expect("Could not transform data node field")
                })
                .collect();

            for json_node in json_nodes {
                let json = json_node.json;
                let value = json!(json.value.trim().to_string());
                result.insert(json.key, value);
            }
        }

        recurse(
            Arc::clone(&meta_context),
            Arc::clone(&graph_node),
            &mut result,
        );

        Ok(serde_json::to_string_pretty(&result).expect("Could not make a JSON string"))
    }
}
