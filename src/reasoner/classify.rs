use std::sync::Arc;
use serde_json::json;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability};
use crate::classification::Classification;

pub async fn classify<R: Reasoner>(
    reasoner: &R,
    meta_context: Arc<MetaContext>
) -> Result<(Classification, ReasonerMetadata), Errors> {

    let system_prompt = get_system_prompt(reasoner, Arc::clone(&meta_context)).await?;
    let user_prompt = meta_context.generate_context_string()?;
    let schema = json!({
        "type": "object",
        "properties": {
            "description": {
                "type": "string",
                "description": "description of web page"
            },
            "structure": {
                "type": "string",
                "description": "technical description of web page"
            },
            "category": {
                "type": "string",
                "description": "categorization of web page"
            },
            "one_word_aliases": {
                "type": "array",
                "description": "array of category aliases",
                "items": {
                    "type": "string",
                    "description": "an alias of the main category"
                }
            },
            "two_word_aliases": {
                "type": "array",
                "description": "array of category aliases",
                "items": {
                    "type": "string",
                    "description": "an alias of the main category"
                }
            }
        },
        "required": ["description", "structure", "category", "one_word_aliases", "two_word_aliases"],
        "additionalproperties": false
    });
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
