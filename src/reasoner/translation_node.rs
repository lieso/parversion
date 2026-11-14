use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::normal_context::NormalContext;
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;
use crate::translation_node::TranslationNode;

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponse {
    /// The exact key from the RIGHT section that this mapping produces
    pub target_field: String,
    /// A single Scheme lambda expression of one argument, `value`, that converts a raw LEFT value into the form of the RIGHT value
    pub program: String,
}

pub async fn translation_node<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_lineage: Lineage,
    left_contexts: Vec<Arc<NormalContext>>,
    right_lineage: Lineage,
    right_contexts: Vec<Arc<NormalContext>>,
) -> Result<(Option<TranslationNode>, ReasonerMetadata), Errors> {
    log::trace!("In translation_node");

    let system_prompt = get_system_prompt(reasoner).await?;
    let schema = serde_json::to_value(schemars::schema_for!(TranslationNodeResponse))
        .expect("Failed to serialise TranslationNodeResponse schema");
    let capability = Capability::Capable;
    let user_prompt = get_user_prompt(
        Arc::clone(&translation_context),
        &left_contexts,
        &right_contexts,
    )?;

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

    unimplemented!()
}

async fn get_system_prompt<R: Reasoner>(reasoner: &R) -> Result<String, Errors> {
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
    translation_context: Arc<RwLock<TranslationContext>>,
    left_contexts: &[Arc<NormalContext>],
    right_contexts: &[Arc<NormalContext>],
) -> Result<String, Errors> {
    let render = |contexts: &[Arc<NormalContext>]| -> String {
        contexts.iter().take(7).fold(String::new(), |acc, context| {
            let data = context
                .data_node
                .fields
                .iter()
                .fold(String::new(), |inner_acc, field| {
                    format!("{}\n{} = {}", inner_acc, field.0, field.1)
                });

            format!("{}\n\nEXAMPLE:\n{}", acc, data)
        })
    };

    let right = render(right_contexts);

    let context_strings: Vec<String> = left_contexts
        .iter()
        .take(11)
        .map(|context| {
            context
                .generate_context_string_translation_node(Arc::clone(&translation_context))
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let left = context_strings.join("\n\n---EXAMPLE SEPARATOR---\n\n");

    Ok(format!(
        r##"
[LEFT]
{}

[RIGHT]
{}
    "##,
        left, right
    ))
}
