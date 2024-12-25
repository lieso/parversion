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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationship {
    Recursion(Recursion),
    Association(Association),
}
