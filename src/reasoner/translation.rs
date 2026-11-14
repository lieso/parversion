use schemars::JsonSchema;
use serde::Deserialize;
use serde_json;
use std::sync::{Arc, RwLock};

use crate::normal_context::NormalContext;
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;
use crate::scheme::eval;

#[derive(Deserialize, JsonSchema)]
pub struct TranslationResponse {
    /// Scheme program that translates from LEFT to RIGHT
    pub forward_program: String,
    /// Scheme program that translates from RIGHT to LEFT
    pub inverse_program: String,
}

pub async fn translation<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_field: String,
    left_contexts: Vec<Arc<NormalContext>>,
    right_field: String,
    right_contexts: Vec<Arc<NormalContext>>,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("In translation");

    let (transformation, reasoner_metadata) = determine_translation(
        reasoner,
        Arc::clone(&translation_context),
        left_field.clone(),
        left_contexts.clone(),
        right_field.clone(),
        right_contexts.clone(),
        Capability::Fast,
    ).await?;

    match validate_transformation(&transformation) {
        Ok(true) => {
            log::info!("Transformation is valid");
            Ok((transformation, reasoner_metadata))
        }
        result => {
            log::warn!("Transformation did not validate");
            log::info!("Escalating to a more advanced model...");

            let (transformation, next_reasoner_metadata) = determine_translation(
                reasoner,
                Arc::clone(&translation_context),
                left_field.clone(),
                left_contexts.clone(),
                right_field.clone(),
                right_contexts.clone(),
                Capability::Capable,
            ).await?;

            let reasoner_metadata = ReasonerMetadata {
                tokens: reasoner_metadata.tokens + next_reasoner_metadata.tokens,
                prompts: [reasoner_metadata.prompts, next_reasoner_metadata.prompts].concat(),
                cost: reasoner_metadata.cost + next_reasoner_metadata.cost,
            };

            Ok((transformation, reasoner_metadata))
        }
    }
}

fn validate_transformation(transformation: &Transformation) -> Result<bool, Errors> {
    match eval(&transformation.forward) {
        Ok(_compiled) => Ok(true),
        Err(error) => {
            log::error!("Failed to compile scheme peogram: {:?}", transformation.forward);
            log::error!("{}", error);
            Ok(false)
        }
    }
}

async fn determine_translation<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_field: String,
    left_contexts: Vec<Arc<NormalContext>>,
    right_field: String,
    right_contexts: Vec<Arc<NormalContext>>,
    capability: Capability,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("determine_translation");

    let system_prompt = get_system_prompt(reasoner).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&translation_context),
        &left_field,
        left_contexts.clone(),
        &right_field,
        right_contexts.clone(),
    ).await?;
    let schema = serde_json::to_value(schemars::schema_for!(TranslationResponse))
        .expect("Failed to serialise TranslationResponse schema");

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   TRANSLATION                                 ║");
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
        .execute::<TranslationResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    let transformation = Transformation {
        id: ID::new(),
        description: "placeholder".to_string(),
        field: left_field.clone(),
        image: right_field.clone(),
        forward: result.forward_program.clone(),
        inverse: result.inverse_program.clone(),
    };

    Ok((transformation, reasoner_metadata))
}

async fn get_system_prompt<R: Reasoner>(reasoner: &R) -> Result<String, Errors> {
    reasoner
        .prompts()
        .get("", "translation")
        .await?
        .ok_or_else(|| {
            Errors::UnavailableSystemPrompt(
                "Expected a translation.txt system prompt in prompts directory".to_string(),
            )
        })
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    translation_context: Arc<RwLock<TranslationContext>>,
    left_field: &str,
    left_contexts: Vec<Arc<NormalContext>>,
    right_field: &str,
    right_contexts: Vec<Arc<NormalContext>>,
) -> Result<String, Errors> {
    unimplemented!()
}
