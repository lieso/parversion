use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::normal_context::NormalContext;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::translation_node::TranslationNode;

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponseItem {
    /// The exact key from the LEFT section that this mapping reads from
    pub source_field: String,
    /// The exact key from the RIGHT section that this mapping produces
    pub target_field: String,
    /// Brief description of what the transformed value represents
    pub description: String,
    /// A single Scheme lambda expression of one argument, `value`, that converts a raw LEFT value into the form of the RIGHT value
    pub program: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponse {
    /// Mappings from LEFT keys to RIGHT keys. Keys that do not align are omitted.
    pub mappings: Vec<TranslationNodeResponseItem>,
}

pub async fn translation_node<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_contexts: Vec<Arc<NormalContext>>,
    right_contexts: Vec<Arc<NormalContext>>
) -> Result<(Option<TranslationNode>, ReasonerMetadata), Errors> {
    log::trace!("In translation_node");

    let system_prompt = get_system_prompt(reasoner).await?;
    let user_prompt = get_user_prompt(
        Arc::clone(&translation_context),
        &left_contexts,
        &right_contexts
    )?;
    let schema = serde_json::to_value(schemars::schema_for!(TranslationNodeResponse))
        .expect("Failed to serialise TranslationNodeResponse schema");
    let capability = Capability::Strong;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   TRANSLATION NODE                            ║");
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
    log::debug!(
        "{}",
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    );
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner
        .execute::<TranslationNodeResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    unimplemented!()
}

async fn get_system_prompt<R: Reasoner>(
    reasoner: &R,
) -> Result<String, Errors> {
    reasoner
        .prompts()
        .get("", "translation_node")
        .await?
        .ok_or_else(|| {
            Errors::UnavailableSystemPrompt(
                "Expected a translation_node.txt system prompt in prompts directory".to_string(),
            )
        })
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
