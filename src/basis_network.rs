use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::NetworkTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub description: String,
    pub subgraph_hash: Hash,
    pub lineage: Lineage,
    pub transformation: NetworkTransformation,
}
