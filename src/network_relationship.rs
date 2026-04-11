use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use serde_json::{json, Value, Map};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::basis_network::{BasisNetwork, NetworkType};
use crate::graph_node::{Graph, GraphNode};
use crate::json_node::JsonNode;
use crate::document::Document;
use crate::llm::LLM;
use crate::xpath::XPath;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRelationshipType {
   Composition,
   OneToMany,
   ParentChild,
}

pub struct NetworkRelationship {}

impl NetworkRelationship {
    pub async fn process_composition(
        meta_context: Arc<RwLock<MetaContext>>,
        network_from: Arc<BasisNetwork>,
        network_to: Arc<BasisNetwork>,
    ) -> Result<(), Errors> {
        log::trace!("In process_composition");

        let snippet = Self::create_network_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&network_from),
            Arc::clone(&network_to),
        )?;

        log::debug!("snippet: {}", snippet);

        let ((forward_xpath, reverse_xpath, merge_variable_name), (tokens,)) = LLM::get_composition_link(
            snippet
        ).await?;

        log::debug!("forward_xpath: {}", forward_xpath);
        log::debug!("reverse_xpath: {}", reverse_xpath);

        let xpath = XPath::from_str(&forward_xpath)?;
        log::debug!("xpath: {:?}", xpath);









        unimplemented!()
    }

    pub async fn process_parent_child(
        meta_context: Arc<RwLock<MetaContext>>,
        network_from: Arc<BasisNetwork>,
        network_to: Arc<BasisNetwork>,
    ) -> Result<(), Errors> {
        log::trace!("In process_parent_child");

        let snippet = Self::create_network_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&network_from),
            Arc::clone(&network_to),
        )?;

        log::debug!("snippet: {}", snippet);

        unimplemented!()
    }

    fn create_network_snippet(
        meta_context: Arc<RwLock<MetaContext>>,
        network_from: Arc<BasisNetwork>,
        network_to: Arc<BasisNetwork>,
    ) -> Result<String, Errors> {
        let graph_root = {
            let lock = read_lock!(meta_context);
            lock.graph_root
                .clone()
                .ok_or(Errors::GraphRootNotProvided)?
        };

        let target_graph_nodes = Self::get_target_graph_nodes(
            Arc::clone(&graph_root),
            &network_from,
            &network_to,
        );

        let mut snippet = String::new();

        Self::get_network_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&graph_root),
            &target_graph_nodes,
            &network_from.subgraph_hash,
            &network_to.subgraph_hash,
            &mut snippet
        );

        Ok(snippet)
    }

    fn get_network_snippet(
        meta_context: Arc<RwLock<MetaContext>>,
        current: Graph,
        target_graph_nodes: &HashSet<ID>,
        network_from_subgraph_hash: &Hash,
        network_to_subgraph_hash: &Hash,
        snippet: &mut String
    )  {
        let lock = read_lock!(current);
        
        let contexts = {
            let lock = read_lock!(meta_context);
            lock.contexts.clone().unwrap()
        };

        let current_context = contexts.get(&lock.id).unwrap();
        let document_node = current_context.document_node.clone();

        let should_render = target_graph_nodes.contains(&lock.id);

        if should_render {
            let (mut a, _b) = read_lock!(document_node).to_string_components();

            let same_network = network_from_subgraph_hash == network_to_subgraph_hash;

            if lock.subgraph_hash == *network_from_subgraph_hash {
                let (marker_prefix, marker_suffix) = if same_network {
                    ("<!-- Target Network: Start -->", "<!-- Target Network: End -->")
                } else {
                    ("<!-- Target Network A: Start -->", "<!-- Target Network A: End -->")
                };

                a = format!("{}{}{}", marker_prefix, a, marker_suffix)
            }

            if !same_network && lock.subgraph_hash == *network_to_subgraph_hash {
                let marker_prefix = "<!-- Target Network B: Start -->";
                let marker_suffix = "<!-- Target Network B: End -->";

                a = format!("{}{}{}", marker_prefix, a, marker_suffix)
            }

            snippet.push_str(&a);
        }

        for child in &lock.children {
            Self::get_network_snippet(
                Arc::clone(&meta_context),
                Arc::clone(&child),
                target_graph_nodes,
                network_from_subgraph_hash,
                network_to_subgraph_hash,
                snippet,
            );
        }

        if should_render {
            let (_a, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(b.as_deref().unwrap_or(""));
        }
    }

    fn get_target_graph_nodes(
        graph_root: Graph,
        network_from: &BasisNetwork,
        network_to: &BasisNetwork,
    ) -> HashSet<ID> {
        let mut target_graph_nodes: HashSet<ID> = HashSet::new();

        let mut network_from_counter: usize = 0;
        let mut network_to_counter: usize = 0;

        let mut found_first_network: bool = false;

        let mut stack: Vec<(Graph, Vec<Graph>)> = vec![(graph_root, vec![])];

        while let Some((current, path)) = stack.pop() {
            let lock = read_lock!(current);

            // We have enough samples of both networks
            if network_from_counter > 3 && network_to_counter > 3 {
                break;
            }

            let from_match = lock.subgraph_hash == network_from.subgraph_hash && network_from_counter <= 3;
            let to_match = lock.subgraph_hash == network_to.subgraph_hash && network_to_counter <= 3;

            if from_match || to_match {
                found_first_network = true;

                if from_match {
                    network_from_counter += 1;
                }

                if to_match {
                    network_to_counter += 1;
                }

                let mut queue = VecDeque::new();
                queue.push_back(Arc::clone(&current));

                while let Some(node) = queue.pop_front() {
                    let id = read_lock!(node).id.clone();

                    target_graph_nodes.insert(id);

                    for child in &read_lock!(node).children {
                        queue.push_back(child.clone());
                    }
                }

                // Ensure path from root node to target network are included
                for node in path.iter() {
                    let id = read_lock!(node).id.clone();
                    target_graph_nodes.insert(id);
                }

            } else if found_first_network {
                let id = read_lock!(current).id.clone();
                target_graph_nodes.insert(id);

                let mut new_path = path.clone();
                new_path.push(Arc::clone(&current));

                for child in lock.children.iter().rev() {
                    stack.push((Arc::clone(child), new_path.clone()));
                }

            } else {
                // Before finding the first network, only traverse without adding
                let mut new_path = path.clone();
                new_path.push(Arc::clone(&current));

                for child in lock.children.iter().rev() {
                    stack.push((Arc::clone(child), new_path.clone()));
                }
            }
        }

        target_graph_nodes
    }

    pub async fn get_relationship_typing(
        meta_context: Arc<RwLock<MetaContext>>,
        networks: Vec<Arc<BasisNetwork>>
    ) -> Result<(Vec<(Arc<BasisNetwork>, Arc<BasisNetwork>, NetworkRelationshipType)>, (u64,)), Errors> {
        log::trace!("In get_relationship_typing");

        let graph_root = {
            let lock = read_lock!(meta_context);
            lock.graph_root
                .clone()
                .ok_or(Errors::GraphRootNotProvided)?
        };

        let mut network_jsons: Vec<(Arc<BasisNetwork>, Vec<String>)> = Vec::new();

        for network in networks.iter() {
            let json_examples = Self::get_network_json(
                Arc::clone(&meta_context),
                network.clone()
            ).await?;

            if json_examples.is_empty() {
                continue;
            }

            network_jsons.push((Arc::clone(network), json_examples));
        }

        let original_document = {
            let lock = read_lock!(meta_context);
            lock.get_original_document()
        };

        let (typed_relationships, (tokens,)) = LLM::identify_relationships(
            Arc::clone(&meta_context),
            original_document,
            network_jsons
        ).await?;

        Ok((typed_relationships, (tokens,)))
    }

    pub async fn get_canonical_networks(
        meta_context: Arc<RwLock<MetaContext>>,
        networks: Vec<Arc<BasisNetwork>>
    ) -> Result<(Vec<BasisNetwork>, (u64,)), Errors> {
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

        let original_document = {
            let lock = read_lock!(meta_context);
            lock.get_original_document()
        };

        let (canonical_ids, (tokens,)) = LLM::check_redundancy(
            Arc::clone(&meta_context),
            original_document,
            all_network_jsons
        ).await?;

        let canonical_networks: Vec<BasisNetwork> = networks
            .iter()
            .filter(|n| canonical_ids.contains(&n.id.to_string()))
            .map(|n| (**n).clone())
            .collect();

        if canonical_networks.is_empty() {
            panic!("No canonical networks found");
        }

        Ok((canonical_networks, (tokens,)))
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
                )?;

                network_jsons.push(json);
            } else {
                for child in &read_lock!(current).children {
                    queue.push_back(child.clone());
                }
            }
        }

        Ok(network_jsons)
    }

    fn process_network(
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
