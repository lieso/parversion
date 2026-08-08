use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_network::BasisNetwork;
use crate::basis_cluster::{NetworkRelationship, NetworkRelationshipType};
use crate::graph_node::{Graph, GraphNode};
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::data_node::DataNode;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// The relationship between two entities extracted from the same document —
/// see the system prompt's decision criteria for how to choose between these.
pub enum RelationshipType {
    Combine,
    Equal,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NetworkRelationshipResponse {
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
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone(),
    )?;
    let schema = serde_json::to_value(schemars::schema_for!(NetworkRelationshipResponse))
        .expect("Failed to serialise NetworkRelationshipResponse schema");
    let capability = Capability::Fast;

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   NETWORK RELATIONSHIP                        ║");
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

    let (result, metadata) = reasoner.execute::<NetworkRelationshipResponse>(
        &capability,
        &system_prompt,
        &user_prompt,
        schema
    ).await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompt_hash: metadata.prompt_hash.clone(),
    };

    match result.relationship_type {
         RelationshipType::Combine => {
             let network_relationship = NetworkRelationship {
                 id: ID::new(),
                 from: left.basis_lineages.clone(),
                 to: right.basis_lineages.clone(),
                 relationship: NetworkRelationshipType::Combine,
             };

             Ok((Some(network_relationship), reasoner_metadata))
         },
         RelationshipType::Equal => {
             let network_relationship = NetworkRelationship {
                 id: ID::new(),
                 from: left.basis_lineages.clone(),
                 to: right.basis_lineages.clone(),
                 relationship: NetworkRelationshipType::Equal,
             };

             Ok((Some(network_relationship), reasoner_metadata))
         },
         RelationshipType::NoRelationship => Ok((None, reasoner_metadata))
    }
}

fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>
) -> Result<String, Errors> {
    let left_context_string = get_user_prompt_basis_network(
        Arc::clone(&normalization_context),
        left,
    )?;
    let right_context_string = get_user_prompt_basis_network(
        Arc::clone(&normalization_context),
        right,
    )?;

    Ok(format!(r##"
[LEFT ENTITY]
{}

[RIGHT ENTITY]
{}
    "##, left_context_string, right_context_string))
}

fn get_user_prompt_basis_network(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_network: Arc<BasisNetwork>,
) -> Result<String, Errors> {
    let contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_network_contexts
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis network contexts not provided in normalization context".to_string())
            })?
            .get(&basis_network.id)
            .map(|contexts| contexts.iter().take(5).cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };

    let result: String = contexts
        .iter()
        .map(|context| {
            let graph_root: Graph = Arc::new(RwLock::new(GraphNode {
                id: ID::new(),
                parents: Vec::new(),
                description: String::new(),
                hash: Hash::new(),
                subgraph_hash: Hash::new(),
                lineage: Lineage::new(),
                children: Vec::new(),
            }));

            let normal_context = Arc::new(basis_network.apply(
                Arc::clone(&normalization_context),
                context.clone(),
                graph_root.clone(),
            )?);

            let mut normal_contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
            let mut contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();

            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
            contexts_lookup.insert(
                read_lock!(normal_context.graph_node).id.clone(),
                Arc::clone(&normal_context),
            );

            let root_normal_context = Arc::new(NormalContext {
                id: ID::new(),
                network_name: None,
                network_description: None,
                data_node: Arc::new(DataNode {
                    id: ID::new(),
                    hash: Hash::new(),
                    lineage: Lineage::new(),
                    fields: HashMap::new(),
                    description: String::new(),
                }),
                graph_node: Arc::clone(&graph_root),
                contexts: Vec::new(),
            });

            normal_contexts.insert(root_normal_context.id.clone(), Arc::clone(&root_normal_context));
            contexts_lookup.insert(
                read_lock!(root_normal_context.graph_node).id.clone(),
                Arc::clone(&root_normal_context),
            );

            let normal_meta_context = NormalMetaContext {
                contexts: normal_contexts,
                graph_root,
                contexts_lookup,
            };

            let normalized = Document::from_normal_meta_context(
                &normal_meta_context,
                &DocumentFormat {
                    format_type: DocumentType::Json,
                    encoding: Some(String::from("UTF-8")),
                    indent: None,
                    line_ending: None,
                    headers: None,
                    wrap_text: None,
                    exclude_nulls: None,
                    custom_delimiter: None,
                },
            )?;

            let original = context.generate_context_string_network_relationship(
                Arc::clone(&normalization_context),
            )?;

            Ok(format!(r##"
[SOURCE DOCUMENT]
{}

[ENTITY]
{}"##, original, normalized.to_string()))
        })
        .collect::<Result<Vec<String>, Errors>>()?
        .join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(result)
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
