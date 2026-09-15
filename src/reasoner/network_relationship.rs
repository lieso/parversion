use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::BasisNetwork;
use crate::basis_graph::NetworkRelationship;

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    unimplemented!()
}
