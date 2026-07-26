use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::transformation::NetworkTransformation;
use crate::graph_node::Graph;

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

impl BasisNetwork {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        context: Arc<Context>
    ) -> Result<Graph, Errors> {
        unimplemented!()
    }
}
