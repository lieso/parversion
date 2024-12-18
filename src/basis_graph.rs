use std::collections::HashMap;

use crate::transformations::{SchemaTransformation};
use crate::id::{ID};
use crate::basis_node::{BasisNode};
use crate::basis_network::{BasisNetwork};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraph {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub json_schema: String,
    pub nodes: HashMap<ID, BasisNode>,
    pub networks: HashMap<ID, BasisNetwork>,
}
