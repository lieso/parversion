use std::collections::HashSet;

use crate::basis_node::{BasisNode};
use crate::hash::{Hash};
use crate::id::{ID};
use crate::transformations::{
    DataNodeFieldsTransform,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub description: String,
    pub relationship: NetworkRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recursion {
    pub lineage: Lineage,
    pub transformation: DataNodeFieldsTransform,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LineageSubgraph {
    pub lineage: Lineage,
    pub subgraph: Hash,
}

pub type Association = Vec<LineageSubgraph>;

pub enum NetworkRelationship {
    Recursion(Recursion),
    Association(Association),
}
