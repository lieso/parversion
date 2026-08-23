use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap, VecDeque};

use crate::prelude::*;
use crate::transformation::NetworkTransformation;
use crate::graph_node::{Graph, GraphNode};
use crate::normal_context::NormalContext;
use crate::data_node::{DataNode, DataNodeFields};
use crate::normal_meta_context::NormalMetaContext;
use crate::basis_node::BasisNode;
use crate::xpath::XPath;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetworkMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub basis_nodes: Vec<Arc<BasisNode>>,
    pub relationships: Vec<Arc<NodeRelationship>>,
    pub transformations: Vec<NetworkTransformation>,
    pub metadata: BasisNetworkMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeRelationshipType {
    Combine { xpath_ltr: String, xpath_rtl: String },
    Equal,
    NoRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeRelationship {
    pub id: ID,
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub relationship_type: NodeRelationshipType,
}

impl BasisNetwork {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        parent: Graph
    ) -> Result<NormalMetaContext, Errors> {
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

        normal_contexts.insert(root_normal_context.id.clone(), Arc::clone(&root_normal_context));
        normal_contexts_lookup.insert(
            read_lock!(root_normal_context.graph_node).id.clone(),
            Arc::clone(&root_normal_context)
        );

        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
        };

        let basis_node_contexts = {
            let lock = read_lock!(normalization_context);
            lock.basis_node_contexts
                .clone()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Basis node contexts not provided in normalization context".to_string())
                })?
        };

        let all_contexts: Vec<(Arc<BasisNode>, Arc<Context>)> = self.basis_nodes
            .iter()
            .flat_map(|basis_node| {
                basis_node_contexts
                    .get(&basis_node.id)
                    .unwrap()
                    .clone()
                    .iter()
                    .map(|context| (basis_node.clone(), context.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut processed_contexts: HashSet<ContextID> = HashSet::new();

        for (basis_node, context) in &all_contexts {
            if processed_contexts.contains(&context.id) {
                continue;
            }

            let mut data_node = {
                if let Some(start_data_node) = basis_node.apply(context.clone())? {
                    start_data_node
                } else {
                    DataNode {
                        id: ID::new(),
                        hash: Hash::new(),
                        lineage: Lineage::new(),
                        fields: DataNodeFields::new(),
                        description: String::new(),
                    }
                }
            };
            processed_contexts.insert(context.id.clone());

            let mut queue: VecDeque<(Arc<Context>, Lineage)> = VecDeque::new();
            queue.push_back((context.clone(), basis_node.lineage.clone()));

            let mut processed_relationships: HashSet<ID> = HashSet::new();

            while let Some((current_context, current_lineage)) = queue.front() {

                let relationships: Vec<Arc<NodeRelationship>> = self.relationships
                    .iter()
                    .filter(|relationship| {
                        !processed_relationships.contains(&relationship.id) && (
                            relationship.left_basis_lineage == *current_lineage ||
                            relationship.right_basis_lineage == *current_lineage
                        )
                    })
                    .cloned()
                    .collect();

                if relationships.is_empty() {
                    queue.pop_front();
                    continue;
                }

                let relationship = relationships.first().unwrap();

                match &relationship.relationship_type {
                    NodeRelationshipType::Combine { xpath_ltr, xpath_rtl } => {
                        let xpath: XPath = {
                            if relationship.left_basis_lineage == *current_lineage {
                                XPath::from_str(&xpath_ltr)?
                            } else {
                                XPath::from_str(&xpath_rtl)?
                            }
                        };

                        if let Some(target_graph_node) = GraphNode::traverse_using_xpath(
                            Arc::clone(&normalization_context),
                            Arc::clone(&current_context.graph_node),
                            &xpath
                        )? {
                            // assumming this is the right context...
                            let target_context = meta_context.contexts_lookup
                                .get(&read_lock!(target_graph_node).id)
                                .cloned()
                                .unwrap();

                            if !processed_contexts.contains(&target_context.id) {
                                // TODO: inefficient
                                let target_pair = all_contexts
                                    .iter()
                                    .find(|(basis_node, context)| {
                                        context.id == target_context.id
                                    });

                                if let Some(target_pair) = target_pair {
                                    let target_basis_node = &target_pair.0;

                                    if let Some(target_data_node) = target_basis_node.apply(target_context.clone())? {
                                        data_node = DataNode::from_data_nodes(vec![
                                            data_node,
                                            target_data_node,
                                        ]);
                                    }

                                    queue.push_back((target_context.clone(), target_basis_node.lineage.clone()));
                                } else {
                                    log::warn!("Could not find target context within current network");
                                }

                                processed_contexts.insert(target_context.id.clone());
                            }
                        } else {
                            log::warn!("Failed to apply xpath: {}", xpath.to_string());
                        }

                        processed_relationships.insert(relationship.id.clone());
                    },
                    NodeRelationshipType::Equal => {
                        unimplemented!()
                    },
                    NodeRelationshipType::NoRelationship => {
                        return Err(Errors::UnexpectedError("Did not expect a NoRelationship here..".to_string()));
                    }
                }
            }




            let data_node = Arc::new(data_node.clone());

            let graph_node = Arc::new(RwLock::new(
                GraphNode::from_data_node(
                    Arc::clone(&data_node),
                    vec![Arc::clone(&parent)]
                )
            ));

            write_lock!(parent).children.push(Arc::clone(&graph_node));

            let normal_context = Arc::new(NormalContext {
                id: ID::new(),
                network_name: Some("placeholdernetworkname".to_string()),
                network_description: Some("placeholderdescription".to_string()),
                data_node,
                graph_node: Arc::clone(&graph_node),
                contexts: Vec::new(),
            });

            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
            normal_contexts_lookup.insert(read_lock!(graph_node).id.clone(), Arc::clone(&normal_context));

        }

        Ok(NormalMetaContext {
            contexts: normal_contexts,
            graph_root: parent,
            contexts_lookup: normal_contexts_lookup
        })
    }

    pub fn _apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        parent: Graph
    ) -> Result<NormalMetaContext, Errors> {

        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
        };

        let basis_node_contexts = {
            let lock = read_lock!(normalization_context);
            lock.basis_node_contexts
                .clone()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Basis node contexts not provided in meta context".to_string())
                })?
        };

        if self.relationships.is_empty() {
            unimplemented!();
        }


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

        normal_contexts.insert(root_normal_context.id.clone(), Arc::clone(&root_normal_context));
        normal_contexts_lookup.insert(
            read_lock!(root_normal_context.graph_node).id.clone(),
            Arc::clone(&root_normal_context)
        );



        if let [first_relationship, remaining_relationship @ ..] = &self.relationships[..] {

            let left_basis_node = self.basis_nodes
                .iter()
                .find(|item| {
                    item.lineage == first_relationship.left_basis_lineage
                })
                .unwrap();
            let right_basis_node = self.basis_nodes
                .iter()
                .find(|item| {
                    item.lineage == first_relationship.right_basis_lineage
                })
                .unwrap();

            let contexts: Vec<Arc<Context>> = basis_node_contexts
                .get(&left_basis_node.id)
                .unwrap()
                .clone();

            match &first_relationship.relationship_type {
                NodeRelationshipType::Combine { xpath_ltr, xpath_rtl } => {
                    let xpath = XPath::from_str(&xpath_ltr)?;

                    for context in contexts {
                        let graph_node = Arc::clone(&context.graph_node);

                        if let Some(target_graph_node) = GraphNode::traverse_using_xpath(
                            Arc::clone(&normalization_context),
                            Arc::clone(&context.graph_node),
                            &xpath
                        )? {
                            let target_context = meta_context.contexts_lookup
                                .get(&read_lock!(target_graph_node).id)
                                .cloned()
                                .unwrap();

                            let left_data_node = left_basis_node.apply(context)?;
                            let right_data_node = right_basis_node.apply(target_context)?;

                            let data_nodes: Vec<Option<DataNode>> = vec![left_data_node, right_data_node];
                            let data_nodes: Vec<DataNode> = data_nodes
                                .into_iter()
                                .flatten()
                                .collect();

                            let combined_data_node = Arc::new(DataNode::from_data_nodes(data_nodes));











                            
                            let graph_node = Arc::new(RwLock::new(
                                GraphNode::from_data_node(Arc::clone(&combined_data_node), vec![Arc::clone(&parent)])
                            ));

                            write_lock!(parent).children.push(Arc::clone(&graph_node));


                            let normal_context = Arc::new(NormalContext {
                                id: ID::new(),
                                network_name: Some("placeholdernetworkname".to_string()),
                                network_description: Some("placeholderdescription".to_string()),
                                data_node: combined_data_node,
                                graph_node: Arc::clone(&graph_node),
                                contexts: Vec::new(),
                            });

                            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                            normal_contexts_lookup.insert(read_lock!(graph_node).id.clone(), Arc::clone(&normal_context));



                        }





                    }

                },
                NodeRelationshipType::Equal => {
                    unimplemented!()
                },
                NodeRelationshipType::NoRelationship => {
                    return Err(Errors::UnexpectedError("Did not expect a NoRelationship here..".to_string()));
                }
            }

        }




        Ok(NormalMetaContext {
            contexts: normal_contexts,
            graph_root: parent,
            contexts_lookup: normal_contexts_lookup
        })
    }
}
