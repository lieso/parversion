use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_field::BasisField;

#[derive(Deserialize, JsonSchema)]
pub struct BasisFieldResponse {
    // Whether the attribute contains meaningful data (true) or is safe to ignore entirely
    // (false)
    pub is_meaningful: bool,
}

pub async fn basis_field<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
    candidate: String
) -> Result<(Option<BasisField>, ReasonerMetadata), Errors> {
    log::trace!("In basis_field");
    
    unimplemented!()
}
