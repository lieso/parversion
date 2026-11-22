use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use super::sampling::sample_random;
use crate::basis_group::{BasisGroup, BasisGroupMetadata};
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata, get_system_prompt, PromptName};

#[derive(Deserialize, JsonSchema)]
pub struct BasisGroupResponse {
    // Whether snippets represent the same semantic content
    pub is_match: bool,
}

pub async fn basis_group<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
    acyclic_lineage: Lineage,
    lineage: Option<Lineage>,
    indexed_lineage: Option<Lineage>,
) -> Result<(Option<BasisGroup>, ReasonerMetadata), Errors> {
    log::trace!("In basis_group");

    assert!(group.len() > 1);

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let hash = read_lock!(group[0].graph_node).hash.clone();

    let document_lineage = read_lock!(meta_context.graph_root).lineage.clone();
    let system_prompt = get_system_prompt(
        reasoner,
        document_lineage,
        PromptName::BasisGroup,
    ).await?;

    let user_prompt = get_user_prompt(Arc::clone(&normalization_context), group).await?;
    let schema = serde_json::to_value(schemars::schema_for!(BasisGroupResponse))
        .expect("Failed to serialise BasisGroupResponse schema");
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
        .execute::<BasisGroupResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash],
        cost: metadata.cost.unwrap_or(0.0),
    };

    if result.is_match {
        let basis_group = BasisGroup {
            id: ID::new(),
            hash,
            acyclic_lineage,
            lineage,
            indexed_lineage,
            metadata: BasisGroupMetadata {
                prompts: reasoner_metadata.prompts.clone(),
            },
        };

        Ok((Some(basis_group), reasoner_metadata))
    } else {
        Ok((None, reasoner_metadata))
    }
}

async fn get_user_prompt(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let context_strings: Vec<String> = group
        .iter()
        .map(|context| {
            context.generate_context_string_basis_group(Arc::clone(&normalization_context))
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let samples = sample_random(context_strings, read_lock!(normalization_context).context_sample_size.clone());
    let merged_samples = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(
        r##""
[Snippets]
{}
"##,
        merged_samples
    ))
}
