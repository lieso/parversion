use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_group::BasisGroup;

#[derive(Deserialize, JsonSchema)]
pub struct BasisGroupResponse {
    pub is_match: bool,
}

pub async fn basis_group<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
) -> Result<(Option<BasisGroup>, ReasonerMetadata), Errors> {
    log::trace!("In basis_group");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;

    unimplemented!()
}

async fn get_system_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    let document_type = meta_context.document_type.to_string().to_lowercase();

    let paths_to_try: Vec<String> = vec![
        format!("{}/{}", document_type, meta_context.acyclic_subgraph_hash.clone()),
        format!("{}", document_type)
    ];

    for path in paths_to_try {
        log::trace!("Searching for prompt with path: {}", path);
        if let Some(system_prompt) = reasoner.prompts().get(&path, "basis_group").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a basis_group.txt system prompt in prompts directory".to_string()))
}
