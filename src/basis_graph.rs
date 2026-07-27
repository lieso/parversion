use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::transformation::{RelationshipTransformation};
use crate::basis_network::BasisNetwork;
use crate::graph_node::{GraphNode, Graph};
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub basis_networks: Hash,
    pub relationships: NetworkRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationship {
    Leaf(Arc<BasisNetwork>),
    Node {
        left: Arc<NetworkRelationship>,
        right: Arc<NetworkRelationship>,
        transformation: RelationshipTransformation,
    }
}

impl NetworkRelationship {
    pub fn collect_basis_networks(
        &self,
        networks: &mut Vec<Arc<BasisNetwork>>
    ) {
        match self {
            NetworkRelationship::Leaf(basis_network) => {
                networks.push(Arc::clone(basis_network));
            }
            NetworkRelationship::Node { left, right, .. } => {
                left.collect_basis_networks(networks);
                right.collect_basis_networks(networks);
            }
        }
    }

    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>
    ) -> Result<NormalMetaContext, Errors> {
        match self {
            NetworkRelationship::Leaf(basis_network) => {
                let contexts = {
                    let lock = read_lock!(normalization_context);
                    lock.basis_network_contexts
                        .as_ref()
                        .ok_or_else(|| {
                            Errors::DeficientNormalizationContextError("Basis network contexts not provided in normalization context".to_string())
                        })?
                        .get(&basis_network.id)
                        .unwrap()
                        .clone()
                };

                let graph_root: Graph = Arc::new(RwLock::new(GraphNode {
                    id: ID::new(),
                    parents: Vec::new(),
                    description: String::new(),
                    hash: Hash::new(),
                    subgraph_hash: Hash::new(),
                    lineage: Lineage::new(),
                    children: Vec::new(),
                }));

                let normalized: Vec<NormalContext> = contexts
                    .iter()
                    .map(|context| {
                        basis_network.apply(
                            Arc::clone(&normalization_context),
                            context.clone(),
                            Arc::clone(&graph_root),
                        )
                    })
                    .collect::<Result<Vec<NormalContext>, Errors>>()?;

                let mut contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
                let mut contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();

                for normal_context in normalized {
                    let normal_context = Arc::new(normal_context);

                    contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                    contexts_lookup.insert(
                        read_lock!(normal_context.graph_node).id.clone(),
                        Arc::clone(&normal_context)
                    );
                }

                let normal_context = Arc::new(NormalContext {
                    id: ID::new(),
                    network_name: None,
                    network_description: None,
                    data_node: Arc::new(DataNode {
                        id: ID::new(),
                        hash: Hash::new(),
                        lineage: Lineage::new(),
                        fields: HashMap::new(),
                        description: String::new(),
                    }),
                    graph_node: Arc::clone(&graph_root),
                });

                contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                contexts_lookup.insert(
                    read_lock!(normal_context.graph_node).id.clone(),
                    Arc::clone(&normal_context)
                );

                let normal_meta_context = NormalMetaContext {
                    contexts,
                    graph_root,
                    contexts_lookup
                };

                Ok(normal_meta_context)
            }
            NetworkRelationship::Node { left, right, transformation } => {
                unimplemented!()
            }
        }
    }
}
