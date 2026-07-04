use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_field::{BasisField, BasisFieldMetadata};

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
    candidate: String
) -> Result<(Option<BasisField>, ReasonerMetadata), Errors> {
    log::trace!("In basis_field");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        group,
        candidate.clone()
    ).await?;
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
    log::debug!("{}", serde_json::to_string_pretty(&schema).unwrap_or_default());
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner.execute::<BasisFieldResponse>(
        &capability,
        &system_prompt,
        &user_prompt,
        schema
    ).await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    if result.is_meaningful {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.clone().unwrap()
        };

        let basis_field = BasisField {
            id: ID::new(),
            acyclic_subgraph_hash: meta_context.acyclic_subgraph_hash.clone(),
            name: candidate.clone(),
            metadata: BasisFieldMetadata {
                prompts: vec![metadata.prompt_hash.clone()]
            }
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
    candidate: String
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let context_strings: Vec<String> = group
        .iter()
        .map(|context| context.generate_context_string(&meta_context))
        .collect::<Result<Vec<String>, Errors>>()?;
    let (embeddings, metadata) = reasoner.embed(context_strings.clone()).await?;
    let samples = most_different(context_strings, &embeddings);
    let merged_samples = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(r##"
[Attribute]
{}

[Snippets]
{}
"##, candidate, merged_samples))
}

fn most_different(candidates: Vec<String>, embeddings: &[Vec<f32>]) -> Vec<String> {
    let n = candidates.len();

    // *************************
    let min_samples = 5;
    let threshold = 0.2;
    let max_samples = 50;
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
    normalization_context: Arc<RwLock<NormalizationContext>>
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "basis_field").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a basis_field.txt system prompt in prompts directory".to_string()))
}
