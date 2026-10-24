use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::normal_context::NormalContext;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::translation_node::TranslationNode;

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponse {
    /// True if the left and right data describe the same kind of data and can be translated between
    pub matches: bool,
}

pub async fn translation_node<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_contexts: Vec<Arc<NormalContext>>,
    right_contexts: Vec<Arc<NormalContext>>
) -> Result<(Option<TranslationNode>, ReasonerMetadata), Errors> {

    let user_prompt = get_user_prompt(
        Arc::clone(&translation_context),
        &left_contexts,
        &right_contexts
    )?;
    log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
    log::debug!("{}", user_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");

    unimplemented!()
}

fn get_user_prompt(
    _translation_context: Arc<RwLock<TranslationContext>>,
    left_contexts: &Vec<Arc<NormalContext>>,
    right_contexts: &Vec<Arc<NormalContext>>
) -> Result<String, Errors> {

    let render = |contexts: &Vec<Arc<NormalContext>>| -> String {
        contexts.iter().take(7).fold(String::new(), |acc, context| {
            let data = context.data_node.fields.iter().fold(String::new(), |inner_acc, field| {
                format!("{}\n{} = {}", inner_acc, field.0, field.1)
            });

            format!("{}\n\nEXAMPLE:\n{}", acc, data)
        })
    };

    let left = render(left_contexts);
    let right = render(right_contexts);

    Ok(format!(r##"
[LEFT]
{}

[RIGHT]
{}
    "##, left, right))
}
