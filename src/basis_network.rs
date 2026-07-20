use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::NetworkTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetworkMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub basis_lineages: Hash,
    pub transformation: NetworkTransformation,
    pub metadata: BasisNetworkMetadata,
}
