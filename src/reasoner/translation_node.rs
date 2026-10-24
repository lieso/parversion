use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::normal_context::NormalContext;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::translation_node::TranslationNode;

pub async fn translation_node<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_contexts: Vec<Arc<NormalContext>>,
    right_contexts: Vec<Arc<NormalContext>>
) -> Result<(Option<TranslationNode>, ReasonerMetadata), Errors> {
    unimplemented!()
}
