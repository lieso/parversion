use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use super::sampling::sample_random;
use crate::basis_field::{BasisField, BasisFieldMetadata};
use crate::document::DocumentType;
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata, get_system_prompt, PromptName};

#[derive(Deserialize, JsonSchema)]
pub struct BasisFieldResponse {
    // Whether the attribute contains meaningful data (true) or is safe to ignore entirely
    // (false)
    pub is_meaningful: bool,
}

pub async fn basis_field<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
    candidate: String,
) -> Result<(Option<BasisField>, ReasonerMetadata), Errors> {
    log::trace!("In basis_field");

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };

    if matches!(meta_context.document_type, DocumentType::Json) {
        let basis_field = BasisField {
            id: ID::new(),
            acyclic_subgraph_hash: meta_context.acyclic_subgraph_hash.clone(),
            name: candidate.clone(),
            metadata: BasisFieldMetadata {
                prompts: Vec::new(),
            },
        };

        let reasoner_metadata = ReasonerMetadata {
            tokens: 0,
            prompts: Vec::new(),
            cost: 0.0,
        };

        return Ok((Some(basis_field), reasoner_metadata));
    }

    let lineage = read_lock!(meta_context.graph_root).lineage.clone();
    let system_prompt = get_system_prompt(
        reasoner,
        lineage,
        PromptName::BasisField,
    ).await?;

    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        group,
        candidate.clone(),
    )
    .await?;
    let schema = serde_json::to_value(schemars::schema_for!(BasisFieldResponse))
        .expect("Failed to serialise BasisFieldResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   BASIS FIELD                                 ║");
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
        .execute::<BasisFieldResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    if result.is_meaningful {
        let basis_field = BasisField {
            id: ID::new(),
            acyclic_subgraph_hash: meta_context.acyclic_subgraph_hash.clone(),
            name: candidate.clone(),
            metadata: BasisFieldMetadata {
                prompts: reasoner_metadata.prompts.clone(),
            },
        };

        Ok((Some(basis_field), reasoner_metadata))
    } else {
        Ok((None, reasoner_metadata))
    }
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
    candidate: String,
) -> Result<String, Errors> {
    let context_strings: Vec<String> = group
        .iter()
        .map(|context| context.generate_context_string(Arc::clone(&normalization_context)))
        .collect::<Result<Vec<String>, Errors>>()?;
    let samples = sample_random(context_strings);
    let merged_samples = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(
        r##"
[Attribute]
{}

[Snippets]
{}
"##,
        candidate, merged_samples
    ))
}
