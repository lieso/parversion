use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};

#[derive(Deserialize, JsonSchema)]
pub struct SchemaInstanceResponse {
    /// The generated instance document
    pub document: String,
}

pub async fn generate_document_from_schema<R: Reasoner>(
    reasoner: &R,
    schema: String,
) -> Result<(String, ReasonerMetadata), Errors> {
    log::trace!("In generate_document_from_schema");

    let system_prompt = get_system_prompt(reasoner).await?;
    let user_prompt = schema;
    let response_schema = serde_json::to_value(schemars::schema_for!(SchemaInstanceResponse))
        .expect("Failed to serialise SchemaInstanceResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   SCHEMA-TO-INSTANCE                          ║");
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
        serde_json::to_string_pretty(&response_schema).unwrap_or_default()
    );
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner
        .execute::<SchemaInstanceResponse>(
            &capability,
            &system_prompt,
            &user_prompt,
            response_schema,
        )
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    Ok((result.document, reasoner_metadata))
}

async fn get_system_prompt<R: Reasoner>(reasoner: &R) -> Result<String, Errors> {
    reasoner
        .prompts()
        .get("", "schema_to_instance")
        .await?
        .ok_or_else(|| {
            Errors::UnavailableSystemPrompt(
                "Expected a schema_to_instance.txt system prompt in prompts directory".to_string(),
            )
        })
}
