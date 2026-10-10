use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisFieldMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisField {
    pub id: ID,
    pub acyclic_subgraph_hash: Hash,
    pub name: String,
    pub metadata: BasisFieldMetadata,
}
