use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::{BasisNetwork, BasisNetworkMetadata};
use crate::transformation::NetworkTransformation;
use crate::graph_node::Graph;
use super::sampling::{pre_sample_context_group, sample_most_different};

#[derive(Deserialize, JsonSchema, Debug)]
pub struct EntityTransformation {
    /// The exact field keys (as shown in [TRANSFORMED NODES]) belonging to this distinct entity
    pub keys: Vec<String>,
    /// snake_case name for this entity
    pub name: String,
    /// Concise description of this entity
    pub description: String,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct BasisNetworkResponse {
    pub entities: Vec<EntityTransformation>,
}

pub async fn basis_network<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_lineages_hash: Hash,
    context_group: Vec<Arc<Context>>,
    basis_lineages: HashSet<BasisLineage>,
) -> Result<(BasisNetwork, ReasonerMetadata), Errors> {
    log::trace!("In basis_network");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        context_group,
        basis_lineages
    ).await?;
    let schema = serde_json::to_value(schemars::schema_for!(BasisNetworkResponse))
        .expect("Failed to serialise BasisNetworkResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   BASIS NETWORK                               ║");
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

    let (result, metadata) = reasoner.execute::<BasisNetworkResponse>(
        &capability,
        &system_prompt,
        &user_prompt,
        schema
    ).await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    let transformations: Vec<NetworkTransformation> = result
        .entities
        .iter()
        .map(|entity| {
            NetworkTransformation {
                id: ID::new(),
                description: entity.description.clone(),
                image: entity.name.clone(),
                keys: entity.keys.iter().cloned().collect(),
            }
        })
        .collect();

    let basis_network = BasisNetwork {
        id: ID::new(),
        basis_lineages: basis_lineages_hash.clone(),
        transformations,
        metadata: BasisNetworkMetadata {
            prompts: vec![reasoner_metadata.prompt_hash.clone()]
        }
    };

    Ok((basis_network, reasoner_metadata))
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    context_group: Vec<Arc<Context>>,
    basis_lineages: HashSet<BasisLineage>
) -> Result<String, Errors> {
    let context_group = pre_sample_context_group(context_group);
    let context_strings: Vec<String> = context_group
        .iter()
        .take(10)
        .map(|context| {
            let relevant_contexts = {
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };
                let context_to_group = {
                    let lock = read_lock!(normalization_context);
                    lock.context_to_group.clone().ok_or(Errors::DeficientNormalizationContextError("'context_to_group' not provided in normalization context".to_string()))?
                };

                let mut contexts: Vec<Arc<Context>> = Vec::new();

                let mut queue: VecDeque<Graph> = VecDeque::new();
                queue.push_back(Arc::clone(&context.graph_node));

                while let Some(node) = queue.pop_front() {
                    let context = contexts_lookup
                        .get(&read_lock!(node).id)
                        .cloned()
                        .unwrap();

                    if let Some(basis_group) = context_to_group.get(&context.id).cloned() {
                        let basis_lineage = basis_group.get_basis_lineage();

                        if basis_lineages.contains(&basis_lineage) {
                            contexts.push(context.clone());
                        } else {
                            if let Some(basis_node) = read_lock!(node).resolve_basis_node(Arc::clone(&normalization_context))? {
                                if !basis_node.transformations.is_empty() {
                                    continue;
                                }
                            }
                        }
                    }

                    for child in &read_lock!(node).children {
                        queue.push_back(Arc::clone(&child));
                    }
                }

                contexts
            };

            context.generate_context_string_basis_network(
                Arc::clone(&normalization_context),
                relevant_contexts
            )
        })
        .collect::<Result<Vec<String>, Errors>>()?;

    //let (embeddings, metadata) = reasoner.embed(context_strings.clone()).await?;
    //let samples = sample_most_different(context_strings, &embeddings);
    let merged_samples = context_strings.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(r##"
[SNIPPETS]
{}
"##, merged_samples))
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "basis_network").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a basis_network.txt system prompt in prompts directory".to_string()))
}
