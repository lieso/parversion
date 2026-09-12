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
    pub name: String,
    pub description: String,
    pub basis_nodes: Vec<Arc<BasisNode>>,
    pub relationships: Vec<Arc<NodeRelationship>>,
    pub transformations: Vec<NetworkTransformation>,
    pub metadata: BasisNetworkMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeRelationshipType {
    Combine { xpath_ltr: String, xpath_rtl: String },
    Equal { xpath_ltr: String, xpath_rtl: String },
    NoRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeRelationship {
    pub id: ID,
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub relationship_type: NodeRelationshipType,
    pub scope_xpath: Option<String>,
    pub centrality_hint: Option<bool>,
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
            contexts_lookup: normal_contexts_lookup
        })
    }

    fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        normal_contexts: &mut HashMap<ID, Arc<NormalContext>>,
        normal_contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
        processed_contexts: &mut HashSet<ContextID>,
        current: Graph,
        parent: Graph
    ) -> Result<(), Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
        };

        let lookup_context_basis_node = {
            let lock = read_lock!(normalization_context);
            lock.context_basis_node.clone().unwrap()
        };

        let context = meta_context.contexts_lookup.get(&read_lock!(current).id).unwrap();

        if !processed_contexts.contains(&context.id) {
            if let Some(basis_node) = lookup_context_basis_node.get(&context.id) {
                let is_element = self.basis_nodes.iter().any(|node| node.id == basis_node.id);

                if is_element {
                    log::info!("Basis node is an element of the network");

                    let normal_context = self.process_network(
                        Arc::clone(&normalization_context),
                        (context.clone(), basis_node.clone()),
                        processed_contexts,
                        Arc::clone(&parent),
                    )?;
                    let normal_context = Arc::new(normal_context);

                    normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                    normal_contexts_lookup.insert(read_lock!(&normal_context.graph_node).id.clone(), Arc::clone(&normal_context));

                    let graph_node = Arc::clone(&normal_context.graph_node);
                    write_lock!(parent).children.push(graph_node.clone());
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
                Arc::clone(&parent)
            )?;
        }

        Ok(())
    }

    fn process_network(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        leader: (Arc<Context>, Arc<BasisNode>),
        processed_contexts: &mut HashSet<ContextID>,
        parent: Graph
    ) -> Result<NormalContext, Errors> {
        log::trace!("In process_network");

        let mut target_contexts: Vec<Arc<Context>> = Vec::new();

        let actual_relationships: Vec<Arc<NodeRelationship>> = self.relationships
            .iter()
            .filter(|relationship| {
                !matches!(relationship.relationship_type, NodeRelationshipType::NoRelationship)
            })
            .cloned()
            .collect();

        let mut queue: VecDeque<(Arc<Context>, Arc<BasisNode>)> = VecDeque::new();
        queue.push_back(leader.clone());

        let mut processed_relationships: HashSet<ID> = HashSet::new();

        while let Some((current_context, current_node)) = queue.pop_front() {
            processed_contexts.insert(current_context.id.clone());

            let current_relationships: Vec<Arc<NodeRelationship>> = actual_relationships
                .iter()
                .filter(|relationship| {
                    !processed_relationships.contains(&relationship.id) && (
                        relationship.left_basis_lineage == current_node.lineage ||
                        relationship.right_basis_lineage == current_node.lineage
                    )
                })
                .cloned()
                .collect();

            for relationship in current_relationships {
                if relationship.left_basis_lineage == relationship.right_basis_lineage {
                    let other_contexts = apply_self_combine(
                        Arc::clone(&normalization_context),
                        current_context.clone(),
                        current_node.clone(),
                        &relationship
                    )?;

                    for context in other_contexts {
                        target_contexts.push(context.clone());
                        processed_contexts.insert(context.id.clone());
                    }

                    continue;
                }

                match &relationship.relationship_type {
                    NodeRelationshipType::Combine { xpath_ltr, xpath_rtl, .. } => {
                        if let Some((next_context, next_node)) = apply_combine(
                            Arc::clone(&normalization_context),
                            current_context.clone(),
                            current_node.clone(),
                            &relationship,
                        )? {
                            target_contexts.push(next_context.clone());
                            queue.push_back((next_context.clone(), next_node));
                        }

                        processed_relationships.insert(relationship.id.clone());
                    },
                    NodeRelationshipType::Equal { xpath_ltr, xpath_rtl, .. } => {
                        if let Some((next_context, next_node)) = apply_combine(
                            Arc::clone(&normalization_context),
                            current_context.clone(),
                            current_node.clone(),
                            &relationship,
                        )? {
                            target_contexts.push(next_context.clone());
                            queue.push_back((next_context.clone(), next_node));
                        }

                        processed_relationships.insert(relationship.id.clone());
                    },
                    NodeRelationshipType::NoRelationship => {
                        panic!("Did not expect a NoRelationship here..");
                    }
                }
            }
        }









        target_contexts.sort_by(|a, b| {
            read_lock!(a.graph_node).preorder_position().cmp(&read_lock!(b.graph_node).preorder_position())
        });












        let data_node = target_contexts.iter().try_fold(DataNode {
            id: ID::new(),
            hash: Hash::new(),
            lineage: Lineage::new(),
            fields: DataNodeFields::new(),
            description: "placeholder".to_string()
        }, |acc, context| -> Result<DataNode, Errors> {
            let basis_node = {
                let lock = read_lock!(normalization_context);
                let lookup = lock.context_basis_node.as_ref().unwrap();
                lookup.get(&context.id).unwrap().clone()
            };

            if let Some(next_data_node) = basis_node.apply(context.clone())? {
                Ok(DataNode::from_data_nodes(vec![
                    acc,
                    next_data_node
                ]))
            } else {
                Ok(acc)
            }
        })?;







        let normal_context = NormalContext {
            id: ID::new(),
            network_name: Some(self.name.clone()),
            network_description: Some(self.description.clone()),
            data_node: Arc::new(data_node.clone()),
            graph_node: Arc::new(RwLock::new(
                GraphNode::from_data_node(
                    Arc::new(data_node.clone()),
                    vec![Arc::clone(&parent)]
                )
            )),
        };


        Ok(normal_context)
    }
}


fn apply_self_combine(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    context: Arc<Context>,
    basis_node: Arc<BasisNode>,
    relationship: &NodeRelationship,
) -> Result<Vec<Arc<Context>>, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    let xpath_str = relationship.scope_xpath.as_ref().unwrap();
    let xpath = XPath::from_str(&xpath_str)?;

    if let Some(target_graph_node) = xpath.traverse(
        Arc::clone(&normalization_context),
        Arc::clone(&context.graph_node),
    )? {
        let lookup_context_basis_node = {
            let lock = read_lock!(normalization_context);
            lock.context_basis_node.as_ref().unwrap().clone()
        };

        let mut matching_contexts = Vec::new();
        let mut queue: VecDeque<Graph> = VecDeque::new();
        let mut visited: HashSet<ID> = HashSet::new();

        queue.push_back(target_graph_node);

        while let Some(current_node) = queue.pop_front() {
            let node_id = read_lock!(current_node).id.clone();
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);

            let current_context = meta_context.contexts_lookup
                .get(&read_lock!(current_node).id)
                .cloned();

            if let Some(ctx) = current_context {
                if let Some(ctx_basis_node) = lookup_context_basis_node.get(&ctx.id) {
                    if ctx_basis_node.id == basis_node.id {
                        matching_contexts.push(ctx);
                    }
                }
            }

            for child in &read_lock!(current_node).children {
                queue.push_back(Arc::clone(child));
            }
        }

        return Ok(matching_contexts);
    }

    Ok(Vec::new())
}

fn apply_combine(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    context: Arc<Context>,
    basis_node: Arc<BasisNode>,
    relationship: &NodeRelationship,
) -> Result<Option<(Arc<Context>, Arc<BasisNode>)>, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    let xpath_str = match &relationship.relationship_type {
        NodeRelationshipType::Combine { xpath_ltr, xpath_rtl, .. } => {
            if relationship.left_basis_lineage == basis_node.lineage {
                xpath_ltr
            } else {
                xpath_rtl
            }
        }
        // TODO: Delete branch
        NodeRelationshipType::Equal { xpath_ltr, xpath_rtl, .. } => {
            if relationship.left_basis_lineage == basis_node.lineage {
                xpath_ltr
            } else {
                xpath_rtl
            }
        }
        _ => return Err(Errors::UnexpectedError("Expected Combine relationship".to_string())),
    };

    let xpath: XPath = XPath::from_str(&xpath_str)?;

    if let Some(target_graph_node) = xpath.traverse(
        Arc::clone(&normalization_context),
        Arc::clone(&context.graph_node),
    )? {
        // assumming this is the right context...
        let target_context = meta_context.contexts_lookup
            .get(&read_lock!(target_graph_node).id)
            .cloned()
            .unwrap();

        let target_basis_node = {
            let lock = read_lock!(normalization_context);
            let lookup = lock.context_basis_node.as_ref().unwrap();

            lookup.get(&target_context.id).cloned()
        };

        if let Some(target_basis_node) = target_basis_node {
            return Ok(Some((target_context, target_basis_node.clone())));
        } else {
            log::warn!("Could not find target context within current network: {}", xpath.to_string());
        }
    }

    Ok(None)
}
