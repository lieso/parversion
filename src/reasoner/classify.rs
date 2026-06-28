use std::sync::Arc;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata};
use crate::classification::Classification;

pub async fn classify<R: Reasoner>(
    reasoner: &R,
    meta_context: Arc<MetaContext>
) -> Result<(Classification, ReasonerMetadata), Errors> {

    let user_prompt = meta_context.generate_context_string()?;


    let document_type = {
        let first_context = meta_context.contexts
            .values()
            .next()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Did not expect MetaContext to have zero contexts.".to_string())
            })?;
        let document_node = read_lock!(first_context.document_node);
        document_node.get_document_type().clone()
    };

    

    let system_prompt_path = format!("root/{:?}", document_type);

    if let Some(system_prompt) = reasoner.prompts().get(&system_prompt_path, "classify.txt").await? {

    } else {
        return Err(Errors::UnavailableSystemPrompt(format!("Expected a classify.txt system prompt at: {}", system_prompt_path)));
    }




    todo!()
}
