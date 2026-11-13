use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use futures::future::try_join_all;
use tokio::task;

use super::sampling::{pre_sample_context_group, sample_most_different};
use crate::basis_group::BasisGroup;
use crate::basis_node::{BasisNode, BasisNodeMetadata};
use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;

#[derive(Deserialize, JsonSchema)]
pub struct BasisNodeResponseItem {
    /// The original field being analyzed (e.g., "text", "href")
    pub source_field: String,
    /// Semantic snake_case identifier reflecting the field's role in the data model
    pub field_name: String,
    /// Brief description of what this data represents, as if documenting an API field
    pub description: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct BasisNodeResponse {
    /// Array of extracted data fields that passed boilerplate and advertisement filters
    pub fields: Vec<BasisNodeResponseItem>,
}

pub async fn basis_node<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_group: Arc<BasisGroup>,
    context_group: Vec<Arc<Context>>,
) -> Result<(BasisNode, ReasonerMetadata), Errors> {
    log::trace!("In basis_node");

    let system_prompt = get_system_prompt(reasoner, Arc::clone(&normalization_context)).await?;
    let user_prompt =
        get_user_prompt(reasoner, Arc::clone(&normalization_context), context_group.clone()).await?;
    let schema = serde_json::to_value(schemars::schema_for!(BasisNodeResponse))
        .expect("Failed to serialise BasisNodeResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   BASIS NODE                                  ║");
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
        .execute::<BasisNodeResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let mut total_tokens: u32 = metadata.input_tokens + metadata.output_tokens;
    let mut total_cost: f64 = metadata.cost.unwrap_or(0.0);
    let mut total_prompts = vec![metadata.prompt_hash.clone()];

    let basis_fields = {
        let lock = read_lock!(normalization_context);
        lock.basis_fields
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(
                    "Basis fields not provided in normalization context".to_string(),
                )
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let mut futures = Vec::new();

    for response_field in result.fields {
        let field = {
            let source = response_field.source_field.trim();
            let after_field_marker = source
                .strip_prefix("FIELD:")
                .or_else(|| source.strip_prefix("FIELD="))
                .unwrap_or(source);
            let field_token = after_field_marker
                .split(',')
                .next()
                .unwrap_or(after_field_marker)
                .trim();

            if field_token.eq_ignore_ascii_case("text") {
                "text".to_string()
            } else {
                field_token.to_string()
            }
        };

        if !basis_fields.iter().any(|basis_field| basis_field.name == field) {
            panic!(
                "Invalid source field in LLM response: parsed field '{}' (raw source_field: '{}') not found in basis_fields: {:?}",
                field,
                response_field.source_field,
                basis_fields.iter().map(|f| &f.name).collect::<Vec<_>>()
            );
        }

        let cloned_reasoner = reasoner.clone();
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_context_group = context_group.clone();
        let image = response_field.field_name;

        futures.push(async move {
            reasoner
                .transformation(cloned_normalization_context, field, image, cloned_context_group)
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

    let basis_node = BasisNode {
        id: ID::new(),
        lineage: basis_group.get_basis_lineage(),
        transformations,
        metadata: BasisNodeMetadata {
            prompts: reasoner_metadata.prompts.clone(),
        },
    };

    Ok((basis_node, reasoner_metadata))
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let basis_fields = {
        let lock = read_lock!(normalization_context);
        lock.basis_fields
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(
                    "Basis fields not provided in normalization context".to_string(),
                )
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let group: Vec<Arc<Context>> = group
        .into_iter()
        .filter(|context| {
            basis_fields
                .iter()
                .any(|field| context.data_node.fields.contains_key(&field.name))
        })
        .collect();

    if group.is_empty() {
        panic!("empty group");
    }

    let group = pre_sample_context_group(group);

    let context_strings: Vec<String> = group
        .iter()
        .map(|context| {
            context.generate_context_string_basis_node(Arc::clone(&normalization_context))
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let (embeddings, _metadata) = reasoner.embed(context_strings.clone()).await?;
    let samples = sample_most_different(context_strings, &embeddings);
    let merged_samples = samples.join("\n\n---OCCURRENCE SEPARATOR---\n\n");

    Ok(format!(
        r##"
[OCCURRENCES]
{}
"##,
        merged_samples
    ))
}

async fn get_system_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
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

    let paths_to_try: Vec<String> = vec![
        format!(
            "{}/{}",
            document_type,
            meta_context.acyclic_subgraph_hash.clone()
        ),
        format!("{}", document_type),
    ];

    for path in paths_to_try {
        log::trace!("Searching for prompt with path: {}", path);
        if let Some(system_prompt) = reasoner.prompts().get(&path, "basis_node").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt(
        "Expected a basis_node.txt system prompt in prompts directory".to_string(),
    ))
}
