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
    if left.id == right.id {
        network_relationship_reflexive(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone()
        ).await
    } else {
        network_relationship_comparative(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone()
        ).await
    }
}

async fn network_relationship_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    network: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    unimplemented!()
}

async fn network_relationship_comparative<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    unimplemented!()
}
