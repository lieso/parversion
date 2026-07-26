use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::{RelationshipTransformation};
use crate::basis_network::BasisNetwork;
use crate::graph_node::Graph;
use crate::normal_context::NormalContext;

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
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>
    ) -> Result<Graph, Errors> {
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

                let normalized: Result<Vec<NormalContext>, Errors> = contexts
                    .iter()
                    .map(|context| {
                        basis_network.apply(
                            Arc::clone(&normalization_context),
                            context.clone()
                        )
                    })
                    .collect();
                let normalized = normalized?;

                //let normal_context = NormalContext {
                //    id: ID::new(),
                //    network_name: None,
                //    network_description: None,
                //    data_node: Arc::new(DataNode::new()),
                //};

                unimplemented!()
            }
            NetworkRelationship::Node { left, right, transformation } => {
                unimplemented!()
            }
        }
    }
}
