use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraph {
    pub id: ID,
    pub lineage: Lineage,
    pub name: String,
    pub description: String,
    pub structure: String,
}
