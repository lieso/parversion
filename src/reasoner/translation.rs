use schemars::JsonSchema;
use serde::Deserialize;
use serde_json;
use std::sync::{Arc, RwLock};

use crate::normal_context::NormalContext;
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;

#[derive(Deserialize, JsonSchema)]
pub struct TransformationResponse {
    /// Scheme program that normalizes a value
    pub forward_program: String,
    /// Scheme program that denormalizes a value
    pub inverse_program: String,
}

pub async fn translation<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_field: String,
    left_contexts: Vec<Arc<NormalContext>>,
    right_field: String,
    right_contexts: Vec<Arc<NormalContext>>,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("In translation");

    unimplemented!()
}
