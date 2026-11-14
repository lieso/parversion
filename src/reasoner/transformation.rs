use schemars::JsonSchema;
use serde::Deserialize;
use serde_json;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;
use crate::scheme::eval;

#[derive(Deserialize, JsonSchema)]
pub struct TransformationResponse {
    /// Scheme program that normalizes a value
    pub forward_program: String,
    /// Scheme program that denormalizes a value
    pub inverse_program: String,
}

pub async fn transformation<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: String,
    image: String,
    description: String,
    contexts: Vec<Arc<Context>>,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("In transformation");

    let (transformation, reasoner_metadata) = determine_transformation(
        reasoner,
        Arc::clone(&normalization_context),
        field.clone(),
        image.clone(),
        description.clone(),
        contexts.clone(),
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

            let (transformation, next_reasoner_metadata) = determine_transformation(
                reasoner,
                Arc::clone(&normalization_context),
                field.clone(),
                image.clone(),
                description.clone(),
                contexts.clone(),
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

async fn determine_transformation<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: String,
    image: String,
    description: String,
    contexts: Vec<Arc<Context>>,
    capability: Capability,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("determine_transformation");

    let system_prompt = get_system_prompt(reasoner).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        &field,
        contexts.clone(),
    ).await?;
    let schema = serde_json::to_value(schemars::schema_for!(TransformationResponse))
        .expect("Failed to serialise TransformationResponse schema");

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   TRANSFORMATION                              ║");
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
        .execute::<TransformationResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    let transformation = Transformation {
        id: ID::new(),
        description: description.clone(),
        field: field.clone(),
        image: image.clone(),
        forward: result.forward_program.clone(),
        inverse: result.inverse_program.clone(),
    };

    Ok((transformation, reasoner_metadata))
}

async fn get_system_prompt<R: Reasoner>(reasoner: &R) -> Result<String, Errors> {
    reasoner
        .prompts()
        .get("", "transformation")
        .await?
        .ok_or_else(|| {
            Errors::UnavailableSystemPrompt(
                "Expected a transformation.txt system prompt in prompts directory".to_string(),
            )
        })
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: &str,
    group: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .clone()
            .ok_or(Errors::DeficientNormalizationContextError(
                "Meta context not provided in normalization context".to_string(),
            ))?
    };

    let document_type = meta_context.document_type.to_string().to_lowercase();

    let origin = meta_context
        .document_metadata
        .origin
        .clone()
        .unwrap_or("Unknown".to_string());


    let context_strings: Vec<String> = group
        .iter()
        .take(3)
        .map(|context| {
            context.generate_context_string_basis_node(Arc::clone(&normalization_context))
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let snippets = context_strings.join("\n\n---SNIPPET SEPARATOR---\n\n");

    let values: Vec<String> = group
        .iter()
        .flat_map(|context| {
            context.data_node.fields.get_all(field)
        })
        .take(17)
        .map(|field_value| field_value.to_string())
        .collect();
    let values = serde_json::to_string_pretty(&values).expect("Could not make a JSON string");

    Ok(format!(r##"
[DOCUMENT TYPE]
{}

[ORIGIN]
{}

[VALUES]
{}

[SNIPPETS]
{}
"##, document_type, origin, values, snippets
    ))
}
