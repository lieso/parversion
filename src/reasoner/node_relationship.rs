use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::NodeRelationship;
use crate::basis_node::BasisNode;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipType {
    Combine,
    Equal,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NodeRelationshipResponse {
    pub relationship_type: RelationshipType,
    pub left_to_right_xpath: Option<String>,
    pub right_to_left_xpath: Option<String>,
}

pub async fn node_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<(NodeRelationship, ReasonerMetadata), Errors> {

    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone()
    ).await?;
    log::info!("user_prompt: {}", user_prompt);
    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;

    unimplemented!()
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<String, Errors> {
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis node contexts not provided in meta context".to_string())
            })?
    };

    fn make_context(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        basis_node: Arc<BasisNode>,
        contexts: Vec<Arc<Context>>
    ) -> Result<String, Errors> {
        contexts.iter().try_fold(String::new(), |acc, context| {
            let context_string = context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                basis_node.clone()
            )?;

            Ok::<String, Errors>(if acc.is_empty() {
                context_string
            } else {
                format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string)
            })
        })
    }

    let left_context_string = make_context(
        Arc::clone(&normalization_context),
        left.clone(),
        basis_node_contexts
            .get(&left.id)
            .unwrap()
            .iter()
            .take(5)
            .cloned()
            .collect()
    )?;

    let right_context_string = make_context(
        Arc::clone(&normalization_context),
        right.clone(),
        basis_node_contexts
            .get(&right.id)
            .unwrap()
            .iter()
            .take(5)
            .cloned()
            .collect()
    )?;

    Ok(format!(r##"
[LEFT]
{}

[RIGHT]
{}
"##, left_context_string, right_context_string))
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "node_relationship").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a node_relationship.txt system prompt in prompts directory".to_string()))
}
