use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_node::{BasisNode, BasisNodeMetadata};
use crate::basis_group::BasisGroup;

#[derive(Deserialize, JsonSchema)]
pub struct BasisNodeResponse {
    // If the data is boilerplate
    pub is_boilerplate: bool,
    // The inferred snake_case variable name
    pub field_name: String,
    // Concise description
    pub description: String,
    // The likely primitive type (string, number, boolean, url, datetime)
    pub data_type: String,
}

pub async fn basis_node<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_group: Arc<BasisGroup>,
    context_group: Vec<Arc<Context>>,
) -> Result<(Option<BasisNode>, ReasonerMetadata), Errors> {
    log::trace!("In basis_node");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;
    let user_prompts = get_user_prompts(
        reasoner,
        Arc::clone(&normalization_context),
        group,
    ).await?;
    let schema = serde_json::to_value(schemars::schema_for!(BasisNodeResponse))
        .expect("Failed to serialise BasisNodeResponse schema");
    let capability = Capability::Fast;
    


}
