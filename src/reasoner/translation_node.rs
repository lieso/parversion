use schemars::JsonSchema;
use futures::future::try_join_all;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::normal_context::NormalContext;
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;
use crate::translation_node::TranslationNode;

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponseItem {
    /// The field from LEFT that matches right_field
    pub left_field: String,
    /// The field from RIGHT that matches left_field
    pub right_field: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TranslationNodeResponse {
    /// Matches from LEFT to RIGHT
    pub matches: Vec<TranslationNodeResponseItem>
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

    let (result, metadata) = reasoner
        .execute::<TranslationNodeResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let mut total_tokens: u32 = metadata.input_tokens + metadata.output_tokens;
    let mut total_cost: f64 = metadata.cost.unwrap_or(0.0);
    let mut total_prompts = vec![metadata.prompt_hash.clone()];

    let mut futures = Vec::new();

    for response_match in result.matches {

        let cloned_translation_context = Arc::clone(&translation_context);
        let cloned_left_contexts = left_contexts.clone();
        let cloned_right_contexts = right_contexts.clone();

        futures.push(async move {
            reasoner
                .translation(
                    cloned_translation_context,
                    response_match.left_field,
                    cloned_left_contexts,
                    response_match.right_field,
                    cloned_right_contexts,
                )
                .await
        });
    }

    let results = try_join_all(futures).await?;

    let mut transformations: Vec<Transformation> = Vec::new();

    for result in results {
        let (transformation, metadata) = result;

        total_tokens += metadata.tokens;
        total_cost += metadata.cost;
        total_prompts.extend(metadata.prompts);

        transformations.push(transformation);
    }

    let reasoner_metadata = ReasonerMetadata {
        tokens: total_tokens,
        prompts: total_prompts,
        cost: total_cost,
    };

    if transformations.is_empty() {
        Ok((None, reasoner_metadata))
    } else {
        let translation_node = TranslationNode {
            id: ID::new(),
            left_lineage: left_lineage.clone(),
            right_lineage: right_lineage.clone(),
            transformations,
        };

        Ok((Some(translation_node), reasoner_metadata))
    }
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
