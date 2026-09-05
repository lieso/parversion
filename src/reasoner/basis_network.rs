use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_node::BasisNode;
use crate::basis_network::{BasisNetwork, BasisNetworkMetadata, NodeRelationship};

#[derive(Deserialize, JsonSchema, Debug)]
pub struct BasisNetworkResponse {
    // The network name
    pub network_name: String,
    // The network description
    pub network_description: String,
}

pub async fn basis_network<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_nodes: Vec<Arc<BasisNode>>,
    relationships: Vec<Arc<NodeRelationship>>,
) -> Result<(BasisNetwork, ReasonerMetadata), Errors> {
    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context),
    ).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        basis_nodes.clone(),
    ).await?;

    let schema = serde_json::to_value(schemars::schema_for!(BasisNetworkResponse))
        .expect("Failed to serialize BasisNetworkResponse schema");
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

    let basis_network = BasisNetwork {
        id: ID::new(),
        name: result.network_name.clone(),
        description: result.network_description.clone(),
        basis_nodes: basis_nodes.clone(),
        relationships: relationships.clone(),
        transformations: Vec::new(),
        metadata: BasisNetworkMetadata {
            prompts: vec![reasoner_metadata.prompt_hash.clone()]
        }
    };

    Ok((basis_network, reasoner_metadata))
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

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_nodes: Vec<Arc<BasisNode>>
) -> Result<String, Errors> {
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis node contexts not provided in normalization context".to_string())
            })?
    };

    let result = basis_nodes
        .iter()
        .try_fold(String::new(), |acc, basis_node| -> Result<String, Errors> {
            let context = basis_node_contexts
                .get(&basis_node.id)
                .unwrap()
                .into_iter()
                .cloned()
                .next()
                .unwrap();

            let context_string = context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                basis_node.clone()
            )?;

            let new_acc = if acc.is_empty() {
                context_string
            } else {
                format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string)
            };

            Ok(new_acc)
        })?;

    Ok(result)
}
