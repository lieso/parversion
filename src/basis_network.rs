use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap, VecDeque};
use rayon::prelude::*;

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
    Combine { xpath_ltr: String, xpath_rtl: String, reachability: bool },
    Equal { xpath_ltr: String, xpath_rtl: String, reachability: bool },
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

        let network_count = self.basis_nodes
            .iter()
            .fold(0, |acc, basis_node| {
                let count = basis_node_contexts
                    .get(&basis_node.id)
                    .unwrap()
                    .len();

                if count > acc {
                    count
                } else {
                    acc
                }
            });
        log::debug!("network_count: {}", network_count);

        let network_size = self.basis_nodes.len();
        log::debug!("network_size: {}", network_size);

        let leader_node = self.basis_nodes
            .iter()
            .find(|basis_node| {
                let count = basis_node_contexts
                    .get(&basis_node.id)
                    .unwrap()
                    .len();

                count == network_count
            })
            .unwrap();
        let leader_contexts = basis_node_contexts
            .get(&leader_node.id)
            .unwrap();

        let actual_relationships: Vec<Arc<NodeRelationship>> = self.relationships
            .iter()
            .filter(|relationship| {
                !matches!(relationship.relationship_type, NodeRelationshipType::NoRelationship)
            })
            .cloned()
            .collect();

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


        let normal_children = leader_contexts
            .into_par_iter()
            .map(|context| -> Result<Arc<NormalContext>, Errors> {
                let mut data_node = {
                    if let Some(start_data_node) = leader_node.apply(context.clone())? {
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

                let mut queue: VecDeque<(Arc<Context>, Lineage)> = VecDeque::new();
                queue.push_back((context.clone(), leader_node.lineage.clone()));

                let mut processed_relationships: HashSet<ID> = HashSet::new();

                while let Some((current_context, current_lineage)) = queue.pop_front() {
                    let current_relationships: Vec<Arc<NodeRelationship>> = actual_relationships
                        .iter()
                        .filter(|relationship| {
                            !processed_relationships.contains(&relationship.id) && (
                                relationship.left_basis_lineage == current_lineage ||
                                relationship.right_basis_lineage == current_lineage
                            )
                        })
                        .cloned()
                        .collect();

                    for relationship in current_relationships {
                        match &relationship.relationship_type {
                            NodeRelationshipType::Combine { xpath_ltr, xpath_rtl, .. } => {
                                if let Some((next_data_node, next_context, next_lineage)) = apply_combine(
                                    Arc::clone(&normalization_context),
                                    data_node.clone(),
                                    current_context.clone(),
                                    &current_lineage,
                                    &relationship,
                                    &all_contexts
                                )? {
                                    data_node = next_data_node;
                                    queue.push_back((next_context, next_lineage));
                                }
                                processed_relationships.insert(relationship.id.clone());
                            },
                            NodeRelationshipType::Equal { xpath_ltr, xpath_rtl, .. } => {
                                if let Some((next_data_node, next_context, next_lineage)) = apply_combine(
                                    Arc::clone(&normalization_context),
                                    data_node.clone(),
                                    current_context.clone(),
                                    &current_lineage,
                                    &relationship,
                                    &all_contexts
                                )? {
                                    data_node = next_data_node;
                                    queue.push_back((next_context, next_lineage));
                                }
                                processed_relationships.insert(relationship.id.clone());
                            },
                            NodeRelationshipType::NoRelationship => {
                                panic!("Did not expect a NoRelationship here..");
                            }
                        }
                    }
                }

                Ok(Arc::new(NormalContext {
                    id: ID::new(),
                    network_name: Some("placeholdernetworkname".to_string()),
                    network_description: Some("placeholderdescription".to_string()),
                    data_node: Arc::new(data_node.clone()),
                    graph_node: Arc::new(RwLock::new(
                        GraphNode::from_data_node(
                            Arc::new(data_node.clone()),
                            vec![Arc::clone(&parent)]
                        )
                    )),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for normal_context in normal_children {
            let graph_node = Arc::clone(&normal_context.graph_node);
            write_lock!(parent).children.push(graph_node.clone());

            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
            normal_contexts_lookup.insert(read_lock!(&graph_node).id.clone(), Arc::clone(&normal_context));
        }

        Ok(NormalMetaContext {
            contexts: normal_contexts,
            graph_root: parent,
            contexts_lookup: normal_contexts_lookup
        })
    }
}

fn apply_combine(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    data_node: DataNode,
    context: Arc<Context>,
    lineage: &Lineage,
    relationship: &NodeRelationship,
    all_contexts: &Vec<(Arc<BasisNode>, Arc<Context>)>,
) -> Result<Option<(DataNode, Arc<Context>, Lineage)>, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    let xpath_str = match &relationship.relationship_type {
        NodeRelationshipType::Combine { xpath_ltr, xpath_rtl, .. } => {
            if relationship.left_basis_lineage == *lineage {
                xpath_ltr
            } else {
                xpath_rtl
            }
        }
        // TODO: Delete branch
        NodeRelationshipType::Equal { xpath_ltr, xpath_rtl, .. } => {
            if relationship.left_basis_lineage == *lineage {
                xpath_ltr
            } else {
                xpath_rtl
            }
        }
        _ => return Err(Errors::UnexpectedError("Expected Combine relationship".to_string())),
    };

    let xpath: XPath = XPath::from_str(&xpath_str)?;

    if let Some(target_graph_node) = GraphNode::traverse_using_xpath(
        Arc::clone(&normalization_context),
        Arc::clone(&context.graph_node),
        &xpath
    )? {
        // assumming this is the right context...
        let target_context = meta_context.contexts_lookup
            .get(&read_lock!(target_graph_node).id)
            .cloned()
            .unwrap();

        // TODO: inefficient
        let target_pair = all_contexts
            .iter()
            .find(|(basis_node, context)| {
                context.id == target_context.id
            });

        if let Some(target_pair) = target_pair {
            let target_basis_node = &target_pair.0;

            if let Some(target_data_node) = target_basis_node.apply(target_context.clone())? {
                let data_node = DataNode::from_data_nodes(vec![
                    data_node,
                    target_data_node,
                ]);

                return Ok(Some((data_node, target_context.clone(), target_basis_node.lineage.clone())));
            }
        } else {
            log::warn!("Could not find target context within current network: {}", xpath.to_string());

            log::debug!("=====================================================================================================");

            let context_string = target_context.generate_context_string(&meta_context, Vec::new())?;
            log::debug!("context_string: {}", context_string);

            log::debug!("=====================================================================================================");

        }
    }

    Ok(None)
}
