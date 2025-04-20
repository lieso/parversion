use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::transformation::{
    DataNodeFieldsTransform,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub description: String,
    pub relationship: NetworkRelationship,
    pub subgraph_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recursion {
    pub lineage: Lineage,
    pub transformation: DataNodeFieldsTransform,
}

pub type Association = Vec<String>; // subgraph hashes

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationship {
    Recursion(Recursion),
    Association(Association),
    Null,
}

impl BasisNetwork {
    pub fn new_null_network(subgraph_hash: &str) -> Self {
        BasisNetwork {
            id: ID::new(),
            description: "Null network".to_string(),
            subgraph_hash: subgraph_hash.to_string().clone(),
            relationship: NetworkRelationship::Null,
        }
    }
}
