use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::BasisNetwork;
use crate::basis_cluster::NetworkRelationship;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipType {
    /// The two networks describe different, complementary parts of the same
    /// entity and should be merged into one object (e.g. one has the invoice
    /// date and amount, the other has the description and customer).
    Combine,
    /// The two networks describe the same entity, extracted twice — same
    /// keys/values pointing at the same conceptual thing.
    Equal,
    /// The two networks describe unrelated entities and should remain separate.
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NetworkRelationshipResponse {
    /// The relationship between the left network and the right network.
    pub relationship_type: RelationshipType,
}

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<(Option<NetworkRelationship>, ReasonerMetadata), Errors> {
    log::trace!("In network_relationship");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context),
    ).await?;

    unimplemented!()
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "network_relationship").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a network_relationship.txt system prompt in prompts directory".to_string()))
}
