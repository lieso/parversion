use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;

pub async fn basis_network<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_nodes: Vec<Arc<BasisNode>>
) -> Result<(BasisNetwork, ReasonerMetadata), Errors> {
    unimplemented!()
}
