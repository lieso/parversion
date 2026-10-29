use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;

use crate::prelude::*;
use crate::normal_context::NormalContext;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::translation_node::TranslationNode;
use crate::data_node::DataNodeFields;
use crate::scheme::{eval, apply};
use crate::transformation::Transformation;

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
    left_contexts: Vec<Arc<NormalContext>>,
    right_contexts: Vec<Arc<NormalContext>>
) -> Result<(Option<TranslationNode>, ReasonerMetadata), Errors> {
    log::trace!("In translation_node");

    let system_prompt = get_system_prompt(reasoner).await?;
    let schema = serde_json::to_value(schemars::schema_for!(TranslationNodeResponse))
        .expect("Failed to serialise TranslationNodeResponse schema");
    let capability = Capability::Capable;

    let fields: HashSet<String> = left_contexts.iter().fold(HashSet::new(), |mut acc, context| {
        for (key, _value) in context.data_node.fields.iter() {
            acc.insert(key.clone());
        }

        acc
    });





    let mut transformations: Vec<Transformation> = Vec::new();
    let mut total_tokens: u32 = 0;
    let mut total_cost: f64 = 0.0;
    let mut total_prompts = Vec::new();





    for field in fields {
        let user_prompt = get_user_prompt(
            Arc::clone(&translation_context),
            &field,
            &left_contexts,
            &right_contexts
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
            .execute::<TranslationNodeResponse>(&capability, &system_prompt, &user_prompt, schema.clone())
            .await?;



        total_tokens += metadata.input_tokens + metadata.output_tokens;
        total_cost += metadata.cost.unwrap_or(0.0);
        total_prompts.push(metadata.prompt_hash.clone());



        if !result.target_field.is_empty() {
            let transformation = Transformation {
                id: ID::new(),
                field: field.clone(),
                image: result.target_field.clone(),
                description: "placeholder".to_string(),
                procedure: Some(result.program.clone()),
            };

            transformations.push(transformation);
        }
    }

    let translation_node = if transformations.is_empty() {
        None
    } else {
        Some(TranslationNode {
            id: ID::new(),
            transformations,
        })
    };

    let reasoner_metadata = ReasonerMetadata {
        tokens: total_tokens,
        cost: total_cost,
        prompts: total_prompts,
    };

    Ok((translation_node, reasoner_metadata))
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
    translation_context: Arc<RwLock<TranslationContext>>,
    field: &String,
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

    let right = render(right_contexts);

    let context_strings: Vec<String> = left_contexts
        .iter()
        .filter(|context| context.data_node.fields.contains_key(&field))
        .take(11)
        .map(|context| {
            context.generate_context_string_translation_node(Arc::clone(&translation_context), &field)
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let left = context_strings.join("\n\n---EXAMPLE SEPARATOR---\n\n");

    Ok(format!(r##"
[LEFT]
{}

[RIGHT]
{}
    "##, left, right))
}
