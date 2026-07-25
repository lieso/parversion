use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::{RelationshipTransformation};
use crate::basis_network::BasisNetwork;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationship {
    Leaf(Arc<BasisNetwork>),
    Merge {
        left: Arc<NetworkRelationship>,
        right: Arc<NetworkRelationship>,
        transformation: RelationshipTransformation,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub basis_networks: Hash,
    pub relationships: NetworkRelationship,
}
