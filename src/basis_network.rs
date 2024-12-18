use std::collections::HashSet;

use crate::basis_node::{BasisNode};
use crate::hash::{Hash};
use crate::id::{ID};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub description: String,
    pub associations: Option<HashSet<Hash>>
    pub recursive_network: Option<Vec<BasisNode>>,
}

