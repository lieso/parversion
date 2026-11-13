use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;

pub async fn transformation<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: String,
    image: String,
    description: String,
    contexts: Vec<Arc<Context>>,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("In transformation");

    unimplemented!()
}
