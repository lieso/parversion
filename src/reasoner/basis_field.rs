use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_field::BasisField;

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


    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let context_strings: Vec<String> = group
        .iter()
        .map(|context| context.generate_context_string(&meta_context))
        .collect::<Result<Vec<String>, Errors>>()?;
    let (embeddings, metadata) = reasoner.embed(context_strings.clone()).await?;
    let sample = most_different(context_strings, &embeddings);




    unimplemented!()
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

        if dist < threshold {
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
