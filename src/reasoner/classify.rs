use std::sync::Arc;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata};
use crate::classification::Classification;

pub async fn classify<R: Reasoner>(
    reasoner: &R,
    meta_context: Arc<MetaContext>
) -> Result<(Classification, ReasonerMetadata), Errors> {

    let system_prompt = get_system_prompt(reasoner, meta_context).await?;
    let user_prompt = meta_context.generate_context_string()?;



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
