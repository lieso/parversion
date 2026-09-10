use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::{NodeRelationship, NodeRelationshipType};
use crate::basis_node::BasisNode;
use crate::graph_node::GraphNode;
use crate::xpath::XPath;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipTypeResponse {
    Combine,
    Equal,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NodeRelationshipResponse {
    // The relationship type between LEFT and RIGHT (e.g. "COMBINE", "EQUAL", "NO_RELATIONSHIP")
    pub relationship_type: RelationshipTypeResponse,
    // The XPath to get from LEFT to RIGHT, if applicable
    pub left_to_right_xpath: Option<String>,
    // The XPath to get from RIGHT to LEFT, if applicable
    pub right_to_left_xpath: Option<String>,
}

pub async fn node_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {

    if left.id == right.id {
        node_relationship_self(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
        ).await
    } else {
        node_relationship_other(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone()
        ).await
    }
}

pub async fn node_relationship_self<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {

    let system_prompt = get_system_prompt_self(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis node contexts not provided in meta context".to_string())
            })?
    };

    let contexts: Vec<Arc<Context>> = basis_node_contexts
        .get(&node.id)
        .unwrap()
        .iter()
        .cloned()
        .collect();

    let user_prompt = get_user_prompt_self(
        reasoner,
        Arc::clone(&normalization_context),
        node.clone(),
        &contexts,
    ).await?;

    let schema = serde_json::to_value(schemars::schema_for!(NodeRelationshipResponse))
        .expect("Failed to serialise NodeRelationshipResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   NODE RELATIONSHIP (SELF)                    ║");
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

    unimplemented!()
}

pub async fn node_relationship_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {
    let mut relationships: Vec<(NodeRelationship, ReasonerMetadata)> = Vec::new();

    let system_prompt = get_system_prompt_other(
        reasoner,
        Arc::clone(&normalization_context)
    ).await?;

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis node contexts not provided in meta context".to_string())
            })?
    };

    let left_contexts: Vec<Arc<Context>> = basis_node_contexts
        .get(&left.id)
        .unwrap()
        .iter()
        .cloned()
        .collect();

    let right_contexts: Vec<Arc<Context>> = basis_node_contexts
        .get(&right.id)
        .unwrap()
        .iter()
        .cloned()
        .collect();

    let user_prompt = get_user_prompt_other(
        reasoner,
        Arc::clone(&normalization_context),
        left.clone(),
        &left_contexts,
        right.clone(),
        &right_contexts,
    ).await?;

    let schema = serde_json::to_value(schemars::schema_for!(NodeRelationshipResponse))
        .expect("Failed to serialise NodeRelationshipResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   NODE RELATIONSHIP (OTHER)                   ║");
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

    let (result, metadata) = reasoner.execute::<NodeRelationshipResponse>(
        &capability,
        &system_prompt,
        &user_prompt,
        schema
    ).await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    let mut relationship_type = {
        match result.relationship_type {
            RelationshipTypeResponse::Combine => {
                NodeRelationshipType::Combine {
                    xpath_ltr: result.left_to_right_xpath.unwrap().clone(),
                    xpath_rtl: result.right_to_left_xpath.unwrap().clone(),
                }
            },
            RelationshipTypeResponse::Equal => {
                NodeRelationshipType::Equal {
                    xpath_ltr: result.left_to_right_xpath.unwrap().clone(),
                    xpath_rtl: result.right_to_left_xpath.unwrap().clone(),
                }
            },
            RelationshipTypeResponse::NoRelationship => {
                NodeRelationshipType::NoRelationship
            },
        }
    };

    let node_relationship = NodeRelationship {
        id: ID::new(),
        left_basis_lineage: left.lineage.clone(),
        right_basis_lineage: right.lineage.clone(),
        relationship_type,
    };

    relationships.push((node_relationship.clone(), reasoner_metadata));

    Ok(relationships)
}

async fn get_user_prompt_self<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Arc<BasisNode>,
    node_contexts: &Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let node_contexts_sample: Vec<Arc<Context>> = node_contexts
        .iter()
        .take(10)
        .cloned()
        .collect();

    let node_context_string = make_context(
        Arc::clone(&normalization_context),
        node.clone(),
        node_contexts_sample.clone(),
    )?;

    Ok(format!(r##"
[NODES]
{}
"##, node_context_string))
}

async fn get_user_prompt_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    left_contexts: &Vec<Arc<Context>>,
    right: Arc<BasisNode>,
    right_contexts: &Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let left_contexts_sample: Vec<Arc<Context>> = left_contexts
        .iter()
        .take(5)
        .cloned()
        .collect();

    let right_contexts_sample: Vec<Arc<Context>> = right_contexts
        .iter()
        .take(5)
        .cloned()
        .collect();


    let left_context_string = make_context(
        Arc::clone(&normalization_context),
        left.clone(),
        left_contexts_sample.clone(),
    )?;

    let right_context_string = make_context(
        Arc::clone(&normalization_context),
        right.clone(),
        right_contexts_sample.clone(),
    )?;

    Ok(format!(r##"
[LEFT]
{}

[RIGHT]
{}
"##, left_context_string, right_context_string))
}

fn make_context(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_node: Arc<BasisNode>,
    contexts: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    contexts.iter().try_fold(String::new(), |acc, context| {
        let context_string = context.generate_context_string_node_relationship(
            Arc::clone(&normalization_context),
            basis_node.clone(),
        )?;

        Ok::<String, Errors>(if acc.is_empty() {
            context_string
        } else {
            format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string)
        })
    })
}

async fn get_system_prompt_self<R: Reasoner>(
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "node_relationship_self").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a node_relationship_self.txt system prompt in prompts directory".to_string()))
}

async fn get_system_prompt_other<R: Reasoner>(
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
        if let Some(system_prompt) = reasoner.prompts().get(&path, "node_relationship_other").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt("Expected a node_relationship_other.txt system prompt in prompts directory".to_string()))
}
