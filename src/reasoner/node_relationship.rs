use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use super::sampling::sample_random;
use crate::basis_node::BasisNode;
use crate::graph_node::Graph;
use crate::node_relationship::{NodeRelationship, NodeRelationshipType};
use crate::prelude::*;
use crate::reasoner::{get_system_prompt, Capability, PromptName, Reasoner, ReasonerMetadata};
use crate::xpath::XPath;

#[derive(Deserialize, Serialize, JsonSchema, Debug)]
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
    MixedContent,
    NoRelationship,
}

#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct NodeRelationshipOtherResponse {
    // The relationship type between LEFT and RIGHT (e.g. "COMBINE", "EQUAL", "MIXED_CONTENT", "NO_RELATIONSHIP")
    pub relationship_type: RelationshipTypeResponse,
    // The XPath to get from LEFT to RIGHT, if applicable
    pub left_to_right_xpath: Option<String>,
    // The XPath to get from RIGHT to LEFT, if applicable
    pub right_to_left_xpath: Option<String>,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct XPathPairResponse {
    // Relative XPath from one instance to other instances of this pattern in the same record
    pub left_to_right_xpath: String,
    // Relative XPath in the opposite direction
    pub right_to_left_xpath: String,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NodeRelationshipSelfResponse {
    // Whether multiple instances of this field can occur within one record and must be combined ("COMBINE"), or whether at most one instance occurs per record ("NO_RELATIONSHIP")
    pub relationship_type: SelfRelationshipTypeResponse,
    // One or more xpath pairs if COMBINE, otherwise an empty list
    #[serde(default)]
    pub xpath_pairs: Vec<XPathPairResponse>,
}

pub async fn node_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<(Vec<NodeRelationship>, ReasonerMetadata), Errors> {
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
) -> Result<(Vec<NodeRelationship>, ReasonerMetadata), Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
        })?
    };
    let contexts: Vec<Arc<Context>> = basis_node_contexts.get(&node.id).unwrap().to_vec();
    let user_prompt =
        get_user_prompt_self(Arc::clone(&normalization_context), node.clone(), &contexts).await?;
    let document_lineage = read_lock!(meta_context.graph_root).lineage.clone();
    let system_prompt =
        get_system_prompt(reasoner, document_lineage, PromptName::NodeRelationshipSelf).await?;

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
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    let relationships: Vec<NodeRelationship> = match result.relationship_type {
        SelfRelationshipTypeResponse::NoRelationship => vec![NodeRelationship {
            id: ID::new(),
            left_basis_lineage: node.lineage.clone(),
            right_basis_lineage: node.lineage.clone(),
            relationship_type: NodeRelationshipType::NoRelationship,
        }],
        SelfRelationshipTypeResponse::Combine | SelfRelationshipTypeResponse::MixedContent => {
            if result.xpath_pairs.is_empty() {
                return Err(Errors::UnexpectedError(
                    "COMBINE or MIXED_CONTENT requires at least one xpath pair".to_string(),
                ));
            }

            let mixed_content = matches!(
                result.relationship_type,
                SelfRelationshipTypeResponse::MixedContent
            );

            result
                .xpath_pairs
                .into_iter()
                .map(|pair| {
                    let (xpath_ltr, xpath_rtl) =
                        (pair.left_to_right_xpath, pair.right_to_left_xpath);

                    NodeRelationship {
                        id: ID::new(),
                        left_basis_lineage: node.lineage.clone(),
                        right_basis_lineage: node.lineage.clone(),
                        relationship_type: if mixed_content {
                            NodeRelationshipType::MixedContent {
                                xpath_ltr,
                                xpath_rtl,
                            }
                        } else {
                            NodeRelationshipType::Combine {
                                xpath_ltr,
                                xpath_rtl,
                            }
                        },
                    }
                })
                .collect()
        }
    };

    Ok((relationships, reasoner_metadata))
}

pub async fn node_relationship_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<(Vec<NodeRelationship>, ReasonerMetadata), Errors> {
    let mut node_relationship: Option<NodeRelationship> = None;
    let mut reasoner_metadata = ReasonerMetadata {
        tokens: 0,
        prompts: Vec::new(),
        cost: 0.0,
    };
    let mut feedback: Option<String> = None;

    let feedback_loop_size = {
        let lock = read_lock!(normalization_context);
        lock.feedback_loop_size
    };

    for _ in 0..feedback_loop_size {
        let (next_node_relationship, next_reasoner_metadata, next_feedback) = determine_node_relationship_other(
            reasoner,
            Arc::clone(&normalization_context),
            Arc::clone(&left),
            Arc::clone(&right),
            Capability::Fast,
            feedback,
        ).await?;

        reasoner_metadata = ReasonerMetadata {
            tokens: reasoner_metadata.tokens + next_reasoner_metadata.tokens,
            prompts: [reasoner_metadata.prompts, next_reasoner_metadata.prompts].concat(),
            cost: reasoner_metadata.cost + next_reasoner_metadata.cost,
        };

        if let Some(node_relationship) = node_relationship {
            if next_node_relationship.relationship_type == node_relationship.relationship_type {
                return Ok((vec![next_node_relationship], reasoner_metadata));
            }
        }

        node_relationship = Some(next_node_relationship);
        feedback = next_feedback;

        let more_feedback = generate_feedback(
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone(),
            &node_relationship.as_ref().unwrap(),
        )?;

        feedback = Some(format!("{}\n{}", feedback.unwrap_or(String::new()), more_feedback));
    }

    Ok((vec![node_relationship.unwrap()], reasoner_metadata))
}

async fn determine_node_relationship_other<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    capability: Capability,
    feedback: Option<String>
) -> Result<(NodeRelationship, ReasonerMetadata, Option<String>), Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let document_lineage = read_lock!(meta_context.graph_root).lineage.clone();
    let system_prompt = get_system_prompt(
        reasoner,
        document_lineage,
        PromptName::NodeRelationshipOther,
    )
    .await?;
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
        })?
    };

    let left_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&left.id).unwrap().to_vec();
    let right_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&right.id).unwrap().to_vec();

    let mut user_prompt = get_user_prompt_other(
        Arc::clone(&normalization_context),
        left.clone(),
        &left_contexts,
        right.clone(),
        &right_contexts,
    )
    .await?;

    if let Some(ref feedback) = feedback {
        user_prompt = format!("{}\n{}", user_prompt, feedback);
    }

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
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    let mut next_feedback = format!(r##"
{}
[FEEDBACK]

[RESPONSE RECEIVED]
{}

"##, feedback.unwrap_or(String::new()), serde_json::to_string_pretty(&result).expect("Could not create json string"));

    let relationship_type = {
        match result.relationship_type {
            RelationshipTypeResponse::NoRelationship => NodeRelationshipType::NoRelationship,
            relationship_type => {
                let xpath_ltr = result.left_to_right_xpath.unwrap_or_else(
                    || {
                        next_feedback.push_str("Expected 'left_to_right_xpath' to be a valid string, received null");
                        String::from(".")
                    }).to_owned();
                let xpath_rtl = result.right_to_left_xpath.unwrap_or_else(
                    || {
                        next_feedback.push_str("Expected 'right_to_left_xpath' to be a valid string, received null");
                        String::from(".")
                    }).to_owned();

                match relationship_type {
                    RelationshipTypeResponse::Combine => NodeRelationshipType::Combine { xpath_ltr, xpath_rtl },
                    RelationshipTypeResponse::Equal => NodeRelationshipType::Equal { xpath_ltr, xpath_rtl },
                    RelationshipTypeResponse::MixedContent => NodeRelationshipType::MixedContent { xpath_ltr, xpath_rtl },
                    RelationshipTypeResponse::NoRelationship => unreachable!(),
                }
            }
        }
    };

    let node_relationship = NodeRelationship {
        id: ID::new(),
        left_basis_lineage: left.lineage.clone(),
        right_basis_lineage: right.lineage.clone(),
        relationship_type,
    };

    Ok((node_relationship, reasoner_metadata, Some(next_feedback)))
}

fn generate_feedback(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    node_relationship: &NodeRelationship,
) -> Result<String, Errors> {
    log::trace!("In generate_feedback");
    
    let mut feedback = String::new();

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
        })?
    };
    let left_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&left.id).unwrap().to_vec();
    let right_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&right.id).unwrap().to_vec();

    let (xpath_ltr, xpath_rtl) = {
        match &node_relationship.relationship_type {
            NodeRelationshipType::NoRelationship => return Ok(feedback),
            NodeRelationshipType::Combine { xpath_ltr, xpath_rtl, .. } => (xpath_ltr, xpath_rtl),
            NodeRelationshipType::Equal { xpath_ltr, xpath_rtl, .. } => (xpath_ltr, xpath_rtl),
            NodeRelationshipType::MixedContent { xpath_ltr, xpath_rtl, .. } => (xpath_ltr, xpath_rtl),
        }
    };

    let xpath_ltr = xpath_ltr.parse()?;
    let xpath_rtl = xpath_rtl.parse()?;

    fn make_feedback(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        contexts: Vec<Arc<Context>>,
        xpath: XPath,
        direction_from: String,
        direction_to: String,
        feedback: &mut String,
    ) -> Result<(), Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        let sample_size = read_lock!(normalization_context).context_sample_size;
        let contexts = sample_random(contexts, sample_size);

        for context in contexts {
            feedback.push_str(&format!("---------[{} TO {} TRAVERSAL]---------\n", direction_from, direction_to));

            let context_string: String = context.generate_context_string(
                Arc::clone(&normalization_context)
            )?;

            feedback.push_str(&format!("[STARTING {} NODE]\n", direction_from));
            feedback.push_str(&format!("{}\n", context_string));

            let target_values = xpath.traverse(
                Arc::clone(&normalization_context),
                Arc::clone(&context.graph_node),
            )?;
            let target_graph_nodes: Vec<Graph> =
                target_values.into_iter().map(|v| v.graph).collect();

            if target_graph_nodes.is_empty() {
                feedback.push_str(&format!("[XPATH TRAVERSAL RESULT]\n"));
                feedback.push_str("Traversal failed to find node\n");
            } else {
                feedback.push_str(&format!("[XPATH TRAVERSAL RESULT]\n"));
                feedback.push_str(&format!("**XPath traversal found {} node(s). Showing result(s) for at most {}:**\n", target_graph_nodes.len(), sample_size));

                for target_graph_node in target_graph_nodes.iter().take(sample_size) {
                    let target_context = meta_context
                        .contexts_lookup
                        .get(&read_lock!(target_graph_node).id)
                        .cloned()
                        .unwrap();

                    let context_string: String = target_context.generate_context_string(
                        Arc::clone(&normalization_context)
                    )?;

                    feedback.push_str(&format!("{}\n", context_string));
                    feedback.push_str(&format!("--- TARGET NODE SEPARATOR ---\n"));
                }
            }
        }

        Ok(())
    }

    make_feedback(Arc::clone(&normalization_context), left_contexts, xpath_ltr, String::from("LEFT"), String::from("RIGHT"), &mut feedback)?;
    make_feedback(Arc::clone(&normalization_context), right_contexts, xpath_rtl, String::from("RIGHT"), String::from("LEFT"), &mut feedback)?;

    Ok(feedback)
}

async fn get_user_prompt_self(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Arc<BasisNode>,
    node_contexts: &[Arc<Context>],
) -> Result<String, Errors> {
    let node_contexts_sample = sample_random(
        node_contexts.to_vec(),
        read_lock!(normalization_context).context_sample_size,
    );

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

async fn get_user_prompt_other(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    left_contexts: &[Arc<Context>],
    right: Arc<BasisNode>,
    right_contexts: &[Arc<Context>],
) -> Result<String, Errors> {
    let left_context_strings: Vec<String> = left_contexts
        .iter()
        .map(|context| {
            context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                left.clone(),
            )
        })
        .collect::<Result<Vec<String>, Errors>>()?;

    let samples = sample_random(
        left_context_strings,
        read_lock!(normalization_context)
            .context_sample_size
            .clone(),
    );
    let left_context_string = samples.join("\n\n---SNIPPET SEPARATOR---\n\n");

    let right_context_strings: Vec<String> = right_contexts
        .iter()
        .map(|context| {
            context.generate_context_string_node_relationship(
                Arc::clone(&normalization_context),
                right.clone(),
            )
        })
        .collect::<Result<Vec<String>, Errors>>()?;

    let samples = sample_random(
        right_context_strings,
        read_lock!(normalization_context)
            .context_sample_size
            .clone(),
    );
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
