use std::sync::Arc;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::classification::Classification;

#[derive(Deserialize, JsonSchema)]
pub struct ClassificationResponse {
    /// Description of document
    pub description: String,
    /// Technical description of document
    pub structure: String,
    /// Categorization of document
    pub category: String,
    /// Array of one-word category aliases
    pub one_word_aliases: Vec<String>,
    /// Array of two-word category aliases
    pub two_word_aliases: Vec<String>,
}

pub async fn classify<R: Reasoner>(
    reasoner: &R,
    meta_context: Arc<MetaContext>
) -> Result<(Classification, ReasonerMetadata), Errors> {
    log::trace!("In classify");

    let system_prompt = get_system_prompt(reasoner, Arc::clone(&meta_context)).await?;
    let user_prompt = meta_context.generate_context_string()?;
    let schema = serde_json::to_value(schemars::schema_for!(ClassificationResponse))
        .expect("Failed to serialise ClassificationResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   CLASSIFY DOCUMENT                          ║");
    log::debug!("║                                                               ║");
    log::debug!("╚═══════════════════════════════════════════════════════════════╝");
    log::debug!("");
    log::debug!("  Capability : {:?}", capability);
    log::debug!("");
    log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
    log::debug!("{}", system_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");
    log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
    log::debug!("{}", user_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");
    log::debug!("┌─── SCHEMA ────────────────────────────────────────────────────┐");
    log::debug!("{}", serde_json::to_string_pretty(&schema).unwrap_or_default());
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, CompletionMetadata) = reasoner.execute::<ClassificationResponse>(
        capability,
        &system_prompt,
        &user_prompt,
        schema,
    ).await?;

    todo!()
}

async fn get_system_prompt<R: Reasoner>(reasoner: &R, meta_context: Arc<MetaContext>) -> Result<String, Errors> {
    let document_type = {
        let first_context = meta_context.contexts
            .values()
            .next()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Did not expect MetaContext to have zero contexts.".to_string())
            })?;
        let document_node = read_lock!(first_context.document_node);
        document_node.get_document_type().to_string().to_lowercase()
    };

    let subgraph_hash = {
        let lock = read_lock!(meta_context.graph_root);
        lock.subgraph_hash.clone().to_string().unwrap()
    };

    let paths_to_try: Vec<String> = vec![
        format!("{}/{}", document_type, subgraph_hash),
        format!("{}", document_type)
    ];

    for path in paths_to_try {
        if let Some(system_prompt) = reasoner.prompts().get(&path, "classify").await? {
            return Ok(system_prompt);
        }
    }
    
    Err(Errors::UnavailableSystemPrompt("Expected a classify.txt system prompt in prompts directory".to_string()))
}
