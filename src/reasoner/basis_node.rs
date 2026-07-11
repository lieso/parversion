use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_node::{BasisNode, BasisNodeMetadata};
use crate::basis_group::BasisGroup;
use crate::data_node::DataNodeFields;

#[derive(Deserialize, JsonSchema)]
pub struct BasisNodeResponseItem {
    /// Semantic snake_case identifier reflecting the field's role in the data model
    pub field_name: String,
    /// Brief description of what this data represents, as if documenting an API field
    pub description: String,
    /// Inferred primitive type (string, number, boolean, url, datetime, etc.)
    pub data_type: String,
    /// Optional. More specific type hint if applicable (email, uuid, slug, iso-date, iso-datetime, currency, percentage, phone, relative-url, absolute-url, base64, hex-color, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
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

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        context_group,
    ).await?;
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
    log::debug!("{}", serde_json::to_string_pretty(&schema).unwrap_or_default());
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    //let (result, metadata) = reasoner.execute::<BasisNodeResponse>(
    //    &capability,
    //    &system_prompt,
    //    &user_prompt,
    //    schema
    //).await?;

    //let reasoner_metadata = ReasonerMetadata {
    //    tokens: metadata.input_tokens + metadata.output_tokens,
    //    prompt_hash: metadata.prompt_hash.clone(),
    //};
    
    unimplemented!()
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    group: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };

    let basis_fields = {
        let lock = read_lock!(normalization_context);
        lock.basis_fields
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis fields not provided in normalization context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let basis_fields_context_string = basis_fields.iter().fold(String::new(), |acc, item| {
        if group.iter().any(|context| {
            context.data_node.fields.contains_key(&item.name)
        }) {
            if item.name == "text" {
                format!("{}\nTEXT", acc)
            } else {
                format!("{}\nATTRIBUTE={}", acc, item.name)
            }
        } else {
            acc
        }
    });

    let extracted_values_string = group.iter().fold(String::new(), |mut acc, item| {
        let fields: &DataNodeFields = &item.data_node.fields;

        for (key, value) in fields {
            if basis_fields.iter().any(|basis_field| {
                basis_field.name == *key
            }) {
                acc = format!("{}\n{}={}", acc, key, value.to_string())
            }
        }

        acc
    });

    let context_strings: Vec<String> = group
        .iter()
        .map(|context| context.generate_context_string(&meta_context))
        .collect::<Result<Vec<String>, Errors>>()?;
    let (embeddings, metadata) = reasoner.embed(context_strings.clone()).await?;
    let samples = most_different(context_strings, &embeddings);
    let merged_samples = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(r##""
[FIELDS TO CONSIDER]
{}

[EXTRACTED VALUES]
{}

[SNIPPETS]
{}
"##, basis_fields_context_string, extracted_values_string, merged_samples))
}

fn most_different(candidates: Vec<String>, embeddings: &[Vec<f32>]) -> Vec<String> {
    let n = candidates.len();

    // *************************
    let min_samples = 5;
    let threshold = 0.2;
    let max_samples = 10;
    // *************************

    if n < min_samples {
        return candidates;
    }

    let mut selected = vec![0usize];
    let mut min_dists: Vec<f32> = embeddings.iter()
        .map(|e| cosine_distance(e, &embeddings[0]))
        .collect();
    min_dists[0] = 0.0;
    
    loop {
        let (next, &dist) = min_dists.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        if dist < threshold && selected.len() >= min_samples {
            break;
        }

        selected.push(next);

        if selected.len() >= max_samples {
            break;
        }

        for (i, d) in min_dists.iter_mut().enumerate() {
            let new_d = cosine_distance(&embeddings[i], &embeddings[next]);
            if new_d < *d {
                *d = new_d;
            }
        }
        min_dists[next] = 0.0;
    }

    selected.iter().map(|index| candidates[*index].clone()).collect()
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    1.0 - dot
}

async fn get_system_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    let document_type = meta_context.document_type.to_string().to_lowercase();

    let paths_to_try: Vec<String> = vec![
        format!("{}/{}", document_type, meta_context.acyclic_subgraph_hash.clone()),
        format!("{}", document_type)
    ];

    for path in paths_to_try {
        log::trace!("Searching for prompt with path: {}", path);
        if let Some(system_prompt) = reasoner.prompts().get(&path, "basis_node").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a basis_node.txt system prompt in prompts directory".to_string()))
}
