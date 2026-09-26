use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use super::sampling::{pre_sample_context_group, sample_most_different};
use crate::basis_network::{NodeRelationship, NodeRelationshipType};
use crate::basis_node::BasisNode;
use crate::graph_node::GraphNode;
use crate::prelude::*;
use crate::reasoner::{Capability, CompletionMetadata, Reasoner, ReasonerMetadata};
use crate::xpath::XPath;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipTypeResponse {
    Combine,
    Equal,
    MixedContent,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelfRelationshipTypeResponse {
    Combine,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CentralityResponse {
    Core,
    Common,
    Occasional,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NodeRelationshipOtherResponse {
    // The relationship type between LEFT and RIGHT (e.g. "COMBINE", "EQUAL", "MIXED_CONTENT", "NO_RELATIONSHIP")
    pub relationship_type: RelationshipTypeResponse,
    // The XPath to get from LEFT to RIGHT, if applicable
    pub left_to_right_xpath: Option<String>,
    // The XPath to get from RIGHT to LEFT, if applicable
    pub right_to_left_xpath: Option<String>,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NodeRelationshipSelfResponse {
    // A short description of the record/entity this field is a member of (e.g. "forum comment", "job listing")
    pub entity_description: String,
    // Whether multiple instances of this field can occur within one record and must be combined ("COMBINE"), or whether at most one instance occurs per record ("NO_RELATIONSHIP")
    pub relationship_type: SelfRelationshipTypeResponse,
    // Brief justification for the relationship_type call
    pub relationship_reasoning: String,
    // Relative XPath from the sampled node up to the smallest ancestor that bounds exactly one record. Required if relationship_type is COMBINE, otherwise null
    pub record_scope_xpath: Option<String>,
    // How essential this field is to its entity: present in virtually every instance ("CORE"), most but not all ("COMMON"), or a minority ("OCCASIONAL")
    pub centrality: CentralityResponse,
    // Brief justification for the centrality call
    pub centrality_reasoning: String,
}

pub async fn node_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {
    if left.id == right.id {
        node_relationship_self(reasoner, Arc::clone(&normalization_context), left.clone()).await
    } else {
        node_relationship_other(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone(),
        )
        .await
    }
}

pub async fn node_relationship_self<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
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
    )
    .await?;

    let system_prompt =
        get_system_prompt_self(reasoner, Arc::clone(&normalization_context)).await?;

    let schema = serde_json::to_value(schemars::schema_for!(NodeRelationshipSelfResponse))
        .expect("Failed to serialise NodeRelationshipSelfResponse schema");
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
    log::debug!(
        "{}",
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    );
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner
        .execute::<NodeRelationshipSelfResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    let mut relationship_type = {
        match result.relationship_type {
            SelfRelationshipTypeResponse::Combine => NodeRelationshipType::Combine {
                xpath_ltr: ".".to_string(),
                xpath_rtl: ".".to_string(),
            },
            SelfRelationshipTypeResponse::NoRelationship => NodeRelationshipType::NoRelationship,
        }
    };

    let mut relationships: Vec<(NodeRelationship, ReasonerMetadata)> = Vec::new();

    let centrality_hint = {
        match result.centrality {
            CentralityResponse::Core => {
                log::info!("=====================================================================================================");
                log::info!("Received Core centrality response");
                log::info!("=====================================================================================================");

                true
            }
            CentralityResponse::Common => {
                log::info!("=====================================================================================================");
                log::info!("Received Common centrality response");
                log::info!("=====================================================================================================");

                false
            }
            CentralityResponse::Occasional => {
                log::info!("=====================================================================================================");
                log::info!("Received Occasional centrality response");
                log::info!("=====================================================================================================");

                false
            }
        }
    };

    let node_relationship = NodeRelationship {
        id: ID::new(),
        left_basis_lineage: node.lineage.clone(),
        right_basis_lineage: node.lineage.clone(),
        relationship_type,
        scope_xpath: result.record_scope_xpath.clone(),
        centrality_hint: Some(centrality_hint.clone()),
    };

    relationships.push((node_relationship.clone(), reasoner_metadata));

    Ok(relationships)
}

pub async fn node_relationship_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<Vec<(NodeRelationship, ReasonerMetadata)>, Errors> {
    let mut relationships: Vec<(NodeRelationship, ReasonerMetadata)> = Vec::new();

    let (node_relationship, reasoner_metadata) = determine_node_relationship_other(
        reasoner,
        Arc::clone(&normalization_context),
        Arc::clone(&left),
        Arc::clone(&right),
        Capability::Fast,
    ).await?;

    match validate_node_relationship(
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone(),
        &node_relationship,
    ) {
        Ok(true) => {
            log::info!("Relationship valid");
            relationships.push((node_relationship.clone(), reasoner_metadata));

            Ok(relationships)
        }
        result => {
            log::warn!("Relationship invalid or error: {:?}", result);
            log::info!("Node relationship did not validate. Escalating to a more advanced model... ");

            let (node_relationship, reasoner_metadata) = determine_node_relationship_other(
                reasoner,
                Arc::clone(&normalization_context),
                Arc::clone(&left),
                Arc::clone(&right),
                Capability::Capable,
            ).await?;

            match validate_node_relationship(
                Arc::clone(&normalization_context),
                left.clone(),
                right.clone(),
                &node_relationship,
            ) {
                Ok(true) => {
                    log::info!("Relationship valid");
                    relationships.push((node_relationship.clone(), reasoner_metadata));

                    Ok(relationships)
                }
                result => {
                    log::warn!("Relationship invalid or error: {:?}", result);
                    log::info!("Node relationship did not validate. Escalating to a more advanced model... ");

                    let (node_relationship, reasoner_metadata) = determine_node_relationship_other(
                        reasoner,
                        Arc::clone(&normalization_context),
                        Arc::clone(&left),
                        Arc::clone(&right),
                        Capability::Strong,
                    ).await?;

                    match validate_node_relationship(
                        Arc::clone(&normalization_context),
                        left.clone(),
                        right.clone(),
                        &node_relationship,
                    ) {
                        Ok(true) => {
                            log::info!("Relationship valid");
                            relationships.push((node_relationship.clone(), reasoner_metadata));

                            Ok(relationships)
                        }
                        result => {
                            log::warn!("Relationship invalid or error: {:?}", result);
                            log::info!("Node relationship did not validate ");
                            panic!();
                        }
                    }


                }
            }
        }
    }
}

async fn determine_node_relationship_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    capability: Capability,
) -> Result<(NodeRelationship, ReasonerMetadata), Errors> {
    let system_prompt =
        get_system_prompt_other(reasoner, Arc::clone(&normalization_context)).await?;

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
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
    )
    .await?;

    let schema = serde_json::to_value(schemars::schema_for!(NodeRelationshipOtherResponse))
        .expect("Failed to serialise NodeRelationshipOtherResponse schema");

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
    log::debug!(
        "{}",
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    );
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner
        .execute::<NodeRelationshipOtherResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    let mut relationship_type = {
        match result.relationship_type {
            RelationshipTypeResponse::Combine => NodeRelationshipType::Combine {
                xpath_ltr: result.left_to_right_xpath.unwrap().clone(),
                xpath_rtl: result.right_to_left_xpath.unwrap().clone(),
            },
            RelationshipTypeResponse::Equal => NodeRelationshipType::Equal {
                xpath_ltr: result.left_to_right_xpath.unwrap().clone(),
                xpath_rtl: result.right_to_left_xpath.unwrap().clone(),
            },
            RelationshipTypeResponse::MixedContent => NodeRelationshipType::MixedContent {
                xpath_ltr: result.left_to_right_xpath.unwrap().clone(),
                xpath_rtl: result.right_to_left_xpath.unwrap().clone(),
            },
            RelationshipTypeResponse::NoRelationship => NodeRelationshipType::NoRelationship,
        }
    };

    let node_relationship = NodeRelationship {
        id: ID::new(),
        left_basis_lineage: left.lineage.clone(),
        right_basis_lineage: right.lineage.clone(),
        relationship_type,
        scope_xpath: None,
        centrality_hint: None,
    };

    Ok((node_relationship, reasoner_metadata))
}

fn validate_node_relationship(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    node_relationship: &NodeRelationship
) -> Result<bool, Errors> {
    log::trace!("In validate_node_relationship");

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .clone()
            .ok_or(Errors::DeficientNormalizationContextError(
                "Meta context not provided in normalization context".to_string(),
            ))?
    };

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
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

    match &node_relationship.relationship_type {
        NodeRelationshipType::Combine {
            xpath_ltr,
            xpath_rtl,
            ..
        } => {
            let xpath: XPath = XPath::from_str(&xpath_ltr)?;

            let coverage_ltr = left_contexts.iter().try_fold(0, |acc, item| -> Result<i32, Errors> {
                let target_graph_nodes = xpath.traverse(
                    Arc::clone(&normalization_context),
                    Arc::clone(&item.graph_node)
                )?;

                if target_graph_nodes.is_empty() {
                    log::warn!(
                        "Could not find target graph nodes within current network: {}",
                        xpath.to_string()
                    );

                    return Ok(acc);
                }

                for target_graph_node in target_graph_nodes {
                    let target_context = meta_context
                        .contexts_lookup
                        .get(&read_lock!(target_graph_node).id)
                        .cloned()
                        .unwrap();

                    let target_basis_node = {
                        let lock = read_lock!(normalization_context);
                        let lookup = lock.context_basis_node.as_ref().unwrap();

                        lookup.get(&target_context.id).cloned()
                    };

                    if let Some(target_basis_node) = target_basis_node {
                        if target_basis_node.id != right.id {
                            log::info!("XPATH LTR did not find the 'right' basis node");
                            return Ok(acc);
                        }
                    }
                }

                Ok(acc + 1)
            })? as f64 / left_contexts.len() as f64;


            log::info!("coverage_ltr: {}", coverage_ltr);

            if coverage_ltr == 0.0 {
                return Ok(false);
            }



            let xpath: XPath = XPath::from_str(&xpath_rtl)?;

            let coverage_rtl = right_contexts.iter().try_fold(0, |acc, item| -> Result<i32, Errors> {
                let target_graph_nodes = xpath.traverse(
                    Arc::clone(&normalization_context),
                    Arc::clone(&item.graph_node)
                )?;

                if target_graph_nodes.is_empty() {
                    log::warn!(
                        "Could not find target graph nodes within current network: {}",
                        xpath.to_string()
                    );

                    return Ok(acc);
                }

                for target_graph_node in target_graph_nodes {
                    let target_context = meta_context
                        .contexts_lookup
                        .get(&read_lock!(target_graph_node).id)
                        .cloned()
                        .unwrap();

                    let target_basis_node = {
                        let lock = read_lock!(normalization_context);
                        let lookup = lock.context_basis_node.as_ref().unwrap();

                        lookup.get(&target_context.id).cloned()
                    };

                    if let Some(target_basis_node) = target_basis_node {
                        if target_basis_node.id != left.id {
                            log::info!("XPATH RTL did not find the 'left' basis node");
                            return Ok(acc);
                        }
                    }
                }

                Ok(acc + 1)
            })? as f64 / right_contexts.len() as f64;


            log::info!("coverage_rtl: {}", coverage_rtl);

            if coverage_rtl == 0.0 {
                return Ok(false);
            }

            Ok(true)
        }
        NodeRelationshipType::Equal {
            xpath_ltr,
            xpath_rtl,
            ..
        } => {
            Ok(true)
        }
        NodeRelationshipType::MixedContent {
            xpath_ltr,
            xpath_rtl,
            ..
        } => {
            Ok(true)
        }
        NodeRelationshipType::NoRelationship => {
            Ok(true)
        }
    }
}

async fn get_user_prompt_self<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Arc<BasisNode>,
    node_contexts: &Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let node_contexts_sample: Vec<Arc<Context>> = node_contexts.iter().take(10).cloned().collect();

    let node_context_string = make_context(
        Arc::clone(&normalization_context),
        node.clone(),
        node_contexts_sample.clone(),
    )?;

    Ok(format!(
        r##"
[NODES]
{}
"##,
        node_context_string
    ))
}

async fn get_user_prompt_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    left_contexts: &Vec<Arc<Context>>,
    right: Arc<BasisNode>,
    right_contexts: &Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let left_contexts_presample = pre_sample_context_group(left_contexts.clone());

    let left_context_strings: Vec<String> = left_contexts_presample
        .iter()
        .map(|context| {
            context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                left.clone(),
            )
        })
        .collect::<Result<Vec<String>, Errors>>()?;

    let (embeddings, metadata) = reasoner.embed(left_context_strings.clone()).await?;
    let samples = sample_most_different(left_context_strings, &embeddings);
    let left_context_string = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    let right_contexts_presample = pre_sample_context_group(right_contexts.clone());

    let right_context_strings: Vec<String> = right_contexts_presample
        .iter()
        .map(|context| {
            context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                right.clone(),
            )
        })
        .collect::<Result<Vec<String>, Errors>>()?;

    let (embeddings, metadata) = reasoner.embed(right_context_strings.clone()).await?;
    let samples = sample_most_different(right_context_strings, &embeddings);
    let right_context_string = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(
        r##"
[LEFT]
{}

[RIGHT]
{}
"##,
        left_context_string, right_context_string
    ))
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
        if let Some(system_prompt) = reasoner
            .prompts()
            .get(&path, "node_relationship_self")
            .await?
        {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt(
        "Expected a node_relationship_self.txt system prompt in prompts directory".to_string(),
    ))
}

async fn get_system_prompt_other<R: Reasoner>(
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
        if let Some(system_prompt) = reasoner
            .prompts()
            .get(&path, "node_relationship_other")
            .await?
        {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt(
        "Expected a node_relationship_other.txt system prompt in prompts directory".to_string(),
    ))
}
