use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_graph::NetworkRelationship;
use crate::transformation::RelationshipTransformation;

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<NetworkRelationship>,
    right: Arc<NetworkRelationship>
) -> Result<(RelationshipTransformation, ReasonerMetadata), Errors> {
    log::trace!("In network_relationship");
    unimplemented!()
}
