use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::basis_network::BasisNetwork;
use crate::traversal::Traversal;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphNode {
    pub id: ID,
    pub parent: Arc<RwLock<BasisGraphNode>>,
    pub basis_network: Arc<BasisNetwork>,
    pub traversal: Traversal,
    pub children: Vec<Arc<RwLock<BasisGraphNode>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub name: Option<String>,
    pub description: Option<String>,
    pub lineage: Lineage,
    pub graph_roots: Vec<BasisGraphNode>,
    pub metadata: BasisGraphMetadata,
}
