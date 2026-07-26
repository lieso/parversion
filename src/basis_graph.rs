use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::{RelationshipTransformation};
use crate::basis_network::BasisNetwork;
use crate::graph_node::Graph;

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
                unimplemented!()
            }
            NetworkRelationship::Node { left, right, transformation } => {
                unimplemented!()
            }
        }
    }
}
