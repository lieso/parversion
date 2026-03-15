use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub description: String,
    pub subgraph_hash: Hash,
    pub lineage: Lineage,
    //pub transformations: Vec<NetworkTransformation>,
}
