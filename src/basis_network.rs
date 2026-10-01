use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use crate::basis_node::BasisNode;
use crate::data_node::{DataNode, DataNodeFields};
use crate::graph_node::{Graph, GraphNode};
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::prelude::*;
use crate::traversal::Traversal;
use crate::xpath::XPath;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetworkMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkShape {
    Reduction,
    Enumeration,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub lineage: Lineage,
    pub shape: NetworkShape,
    pub basis_nodes: Vec<Arc<BasisNode>>,
    pub traversals: Vec<Traversal>,
    pub metadata: BasisNetworkMetadata,
}

impl BasisNetwork {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        parent: Graph,
    ) -> Result<NormalMetaContext, Errors> {
        log::trace!("In apply()");

        let mut normal_contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
        let mut normal_contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();

        let root_normal_context = Arc::new(NormalContext {
            id: ID::new(),
            network_name: None,
            network_description: None,
            data_node: Arc::new(DataNode {
                id: ID::new(),
                hash: Hash::new(),
                lineage: Lineage::new(),
                fields: DataNodeFields::new(),
                description: String::new(),
            }),
            graph_node: Arc::clone(&parent),
            contexts: Vec::new(),
        });

        normal_contexts.insert(
            root_normal_context.id.clone(),
            Arc::clone(&root_normal_context),
        );
        normal_contexts_lookup.insert(
            read_lock!(root_normal_context.graph_node).id.clone(),
            Arc::clone(&root_normal_context),
        );

        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        self.traverse(
            Arc::clone(&normalization_context),
            &mut normal_contexts,
            &mut normal_contexts_lookup,
            &mut HashSet::new(),
            Arc::clone(&meta_context.graph_root),
            Arc::clone(&parent),
        )?;

        Ok(NormalMetaContext {
            contexts: normal_contexts,
            graph_root: parent,
            contexts_lookup: normal_contexts_lookup,
        })
    }

    pub fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        normal_contexts: &mut HashMap<ID, Arc<NormalContext>>,
        normal_contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
        processed_contexts: &mut HashSet<ContextID>,
        current: Graph,
        parent: Graph,
    ) -> Result<(), Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        let lookup_context_basis_node = {
            let lock = read_lock!(normalization_context);
            lock.context_basis_node.clone().unwrap()
        };

        let context = meta_context
            .contexts_lookup
            .get(&read_lock!(current).id)
            .unwrap();

        if !processed_contexts.contains(&context.id) {
            if let Some(basis_node) = lookup_context_basis_node.get(&context.id) {
                let is_element = self.basis_nodes.iter().any(|node| node.id == basis_node.id);

                if is_element {
                    log::info!("Basis node is an element of the network");

                    let normal_context = self.process_network(
                        Arc::clone(&normalization_context),
                        (context.clone(), basis_node.clone()),
                        Arc::clone(&parent),
                        normal_contexts,
                        normal_contexts_lookup,
                        processed_contexts,
                    )?;
                }
            }
        }

        for child in &read_lock!(current).children {
            self.traverse(
                Arc::clone(&normalization_context),
                normal_contexts,
                normal_contexts_lookup,
                processed_contexts,
                Arc::clone(&child),
                Arc::clone(&parent),
            )?;
        }

        Ok(())
    }

    fn process_network(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        leader: (Arc<Context>, Arc<BasisNode>),
        parent: Graph,
        normal_contexts: &mut HashMap<ID, Arc<NormalContext>>,
        normal_contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
        processed_contexts: &mut HashSet<ContextID>,
    ) -> Result<Arc<NormalContext>, Errors> {
        log::trace!("In process_network");

        let graph_node = Arc::new(RwLock::new(GraphNode {
            id: ID::new(),
            parents: vec![Arc::clone(&parent)],
            description: "placeholder".to_string(),
            hash: Hash::new(),
            subgraph_hash: Hash::new(),
            lineage: Lineage::new(),
            children: Vec::new(),
        }));

        let mut network_contexts: Vec<Arc<Context>> = vec![leader.0.clone()];
        processed_contexts.insert(leader.0.id.clone());

        let mut queue: VecDeque<(Arc<Context>, Arc<BasisNode>)> = VecDeque::new();
        queue.push_back(leader.clone());

        let mut processed_traversals: HashSet<ID> = HashSet::new();

        while let Some((current_context, current_node)) = queue.pop_front() {
            let traversals: Vec<Traversal> = self.traversals
                .iter()
                .filter(|traversal| {
                    !processed_traversals.contains(&traversal.id) && (
                        traversal.left_basis_lineage == current_node.lineage ||
                        traversal.right_basis_lineage == current_node.lineage
                    )
                })
                .cloned()
                .collect();

            for traversal in traversals {
                let next_contexts = self.apply_traversal(
                    Arc::clone(&normalization_context),
                    current_context.clone(),
                    current_node.clone(),
                    &traversal,
                )?;

                if next_contexts.iter().all(|(context, _)| {
                    processed_contexts.contains(&context.id)
                }) {
                    processed_traversals.insert(traversal.id.clone());
                } else {
                    for (next_context, next_node) in next_contexts {
                        let is_element = self
                            .basis_nodes
                            .iter()
                            .any(|basis_node| basis_node.id == next_node.id);

                        if is_element {
                            processed_contexts.insert(next_context.id.clone());
                            network_contexts.push(next_context.clone());
                            queue.push_back((next_context.clone(), next_node));
                        } else {
                            let basis_networks = {
                                let lock = read_lock!(normalization_context);
                                lock.basis_networks
                                    .as_ref()
                                    .ok_or_else(|| {
                                        Errors::DeficientNormalizationContextError(
                                            "Basis networks not provided in normalization context".to_string(),
                                        )
                                    })?
                                    .values()
                                    .cloned()
                                    .collect::<Vec<Arc<BasisNetwork>>>()
                            };

                            let next_network = basis_networks
                                .iter()
                                .find(|network| {
                                    network.basis_nodes.iter().any(|basis_node| basis_node.id == next_node.id)
                                })
                                .cloned();

                            if let Some(next_network) = next_network {
                                log::info!("Detected network boundary");

                                next_network.traverse(
                                    Arc::clone(&normalization_context),
                                    normal_contexts,
                                    normal_contexts_lookup,
                                    processed_contexts,
                                    next_context.graph_node.clone(),
                                    graph_node.clone()
                                )?;
                            } else {
                                log::warn!("Traversed to a basis node outside the current network, but has not been placed in any BasisNetwork");
                            }
                        }
                    }
                }
            }
        }

        let existing_network: Option<Arc<NormalContext>> = network_contexts
            .iter()
            .find_map(|context| normal_contexts_lookup.get(&context.id).cloned());

        if let Some(ref existing_network) = existing_network {
            network_contexts.extend(existing_network.contexts.clone());
        }

        match self.shape {
            NetworkShape::Reduction => {
                Ok(self.reduce(
                    Arc::clone(&parent),
                    Arc::clone(&graph_node),
                    Arc::clone(&normalization_context),
                    network_contexts,
                    normal_contexts,
                    normal_contexts_lookup,
                )?)
            }
            NetworkShape::Enumeration => {
                Ok(self.enumerate(
                    Arc::clone(&parent),
                    Arc::clone(&graph_node),
                    Arc::clone(&normalization_context),
                    network_contexts,
                    normal_contexts,
                    normal_contexts_lookup,
                )?)
            }
        }
    }

    fn enumerate(
        &self,
        parent: Graph,
        graph_node: Arc<RwLock<GraphNode>>,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        network_contexts: Vec<Arc<Context>>,
        normal_contexts: &mut HashMap<ID, Arc<NormalContext>>,
        normal_contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
    ) -> Result<Arc<NormalContext>, Errors> {
        log::trace!("In enumerate");

        let container_context = Arc::new(NormalContext {
            id: ID::new(),
            network_name: Some(self.name.clone()),
            network_description: Some(self.description.clone()),
            data_node: Arc::new(DataNode {
                id: ID::new(),
                hash: Hash::new(),
                lineage: Lineage::new(),
                fields: DataNodeFields::new(),
                description: "placeholder".to_string(),
            }),
            graph_node: Arc::clone(&graph_node),
            contexts: vec![],
        });

        normal_contexts.insert(container_context.id.clone(), Arc::clone(&container_context));
        normal_contexts_lookup.insert(
            read_lock!(&container_context.graph_node).id.clone(),
            Arc::clone(&container_context),
        );
        normal_contexts_lookup.insert(
            container_context.id.clone(),
            Arc::clone(&container_context),
        );

        write_lock!(parent).children.push(container_context.graph_node.clone());
        
        // TODO: sort by document order 
        for (index, network_context) in network_contexts.into_iter().enumerate() {
            let basis_node = {
                let lock = read_lock!(normalization_context);
                let lookup = lock.context_basis_node.as_ref().unwrap();
                lookup.get(&network_context.id).unwrap().clone()
            };

            if let Some(data_node) = basis_node.apply(network_context.clone())? {
                let normal_context = Arc::new(NormalContext {
                    id: ID::new(),
                    network_name: Some(format!("{}", index)),
                    network_description: Some("placeholder description".to_string()),
                    data_node: Arc::new(data_node.clone()),
                    graph_node: Arc::new(RwLock::new(GraphNode::from_data_node(
                        Arc::new(data_node.clone()),
                        vec![Arc::clone(&container_context.graph_node)],
                    ))),
                    contexts: vec![network_context.clone()]
                });

                normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                for context in &normal_context.contexts {
                    normal_contexts_lookup.insert(context.id.clone(), Arc::clone(&normal_context));
                }
                normal_contexts_lookup.insert(
                    read_lock!(normal_context.graph_node).id.clone(),
                    Arc::clone(&normal_context),
                );

                write_lock!(container_context.graph_node).children.push(normal_context.graph_node.clone());
            }
        }

        Ok(container_context.clone())
    }

    fn reduce(
        &self,
        parent: Graph,
        graph_node: Arc<RwLock<GraphNode>>,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        network_contexts: Vec<Arc<Context>>,
        normal_contexts: &mut HashMap<ID, Arc<NormalContext>>,
        normal_contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
    ) -> Result<Arc<NormalContext>, Errors> {
        log::trace!("In reduce");

        let data_node = network_contexts.iter().try_fold(
            DataNode {
                id: ID::new(),
                hash: Hash::new(),
                lineage: Lineage::new(),
                fields: DataNodeFields::new(),
                description: "placeholder".to_string(),
            },
            |acc, context| -> Result<DataNode, Errors> {
                let basis_node = {
                    let lock = read_lock!(normalization_context);
                    let lookup = lock.context_basis_node.as_ref().unwrap();
                    lookup.get(&context.id).unwrap().clone()
                };

                if let Some(next_data_node) = basis_node.apply(context.clone())? {
                    Ok(DataNode::from_data_nodes(vec![acc, next_data_node]))
                } else {
                    Ok(acc)
                }
            }
        )?;

        let normal_context = Arc::new(NormalContext {
            id: ID::new(),
            network_name: Some(self.name.clone()),
            network_description: Some(self.description.clone()),
            data_node: Arc::new(data_node.clone()),
            graph_node: Arc::clone(&graph_node),
            contexts: network_contexts.clone(),
        });

        let existing_network: Option<Arc<NormalContext>> = network_contexts
            .iter()
            .find_map(|context| normal_contexts_lookup.get(&context.id).cloned());

        if let Some(existing_network) = existing_network {
            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));

            for context in &normal_context.contexts {
                normal_contexts_lookup.insert(context.id.clone(), Arc::clone(&normal_context));
            }

            normal_contexts_lookup.insert(
                read_lock!(&normal_context.graph_node).id.clone(),
                Arc::clone(&normal_context),
            );
        } else {
            for context in &normal_context.contexts {
                normal_contexts_lookup.insert(context.id.clone(), Arc::clone(&normal_context));
            }

            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
            normal_contexts_lookup.insert(
                read_lock!(&normal_context.graph_node).id.clone(),
                Arc::clone(&normal_context),
            );

            let graph_node = Arc::clone(&normal_context.graph_node);
            write_lock!(parent).children.push(graph_node.clone());
        }

        Ok(normal_context.clone())
    }

    fn apply_traversal(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        context: Arc<Context>,
        basis_node: Arc<BasisNode>,
        traversal: &Traversal,
    ) -> Result<Vec<(Arc<Context>, Arc<BasisNode>)>, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        let xpath_str = {
            if traversal.left_basis_lineage == basis_node.lineage {
                traversal.xpath_ltr.clone()
            } else {
                traversal.xpath_rtl.clone()
            }
        };

        let xpath: XPath = XPath::from_str(&xpath_str)?;

        let target_graph_nodes = xpath.traverse(
            Arc::clone(&normalization_context),
            Arc::clone(&context.graph_node),
        )?;

        if target_graph_nodes.is_empty() {
            log::warn!(
                "Could not find target graph nodes within current network: {}",
                xpath.to_string()
            );
        }

        let mut next_contexts: Vec<(Arc<Context>, Arc<BasisNode>)> = Vec::new();

        for target_graph_node in target_graph_nodes {
            let target_context = meta_context
                .contexts_lookup
                .get(&read_lock!(target_graph_node).id)
                .cloned()
                .unwrap();

            let target_basis_node = {
                let lock = read_lock!(normalization_context);
                let lookup = lock.context_basis_node.as_ref().unwrap();

                lookup.get(&target_context.id).cloned()
            };

            if let Some(target_basis_node) = target_basis_node {
                let expected_lineage = {
                    let is_left = traversal.left_basis_lineage == basis_node.lineage;

                    if is_left {
                        &traversal.right_basis_lineage
                    } else {
                        &traversal.left_basis_lineage
                    }
                };

                if target_basis_node.lineage == *expected_lineage {
                    next_contexts.push((target_context, target_basis_node.clone()));
                } else {
                    log::warn!("xpath located basis node in that does not correspond to the traversal being applied: {}", xpath.to_string());
                }
            } else {
                log::warn!("xpath located context that does not correspond to a basis node in any network: {}", xpath.to_string());
            }
        }

        Ok(next_contexts)
    }
}
