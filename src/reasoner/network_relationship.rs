use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time::sleep;
use std::time::Duration;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_graph::NetworkRelationship;
use crate::transformation::RelationshipTransformation;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::basis_network::BasisNetwork;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipType {
    Merge,
    Combine,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NetworkRelationshipResponse {
    pub relationship_type: RelationshipType,
    pub canonical_network_name: Option<String>,
}

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<NetworkRelationship>,
    right: Arc<NetworkRelationship>
) -> Result<(RelationshipTransformation, ReasonerMetadata), Errors> {
    log::trace!("In network_relationship");

    let system_prompt = get_system_prompt(
        reasoner,
        Arc::clone(&normalization_context),
    ).await?;
    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone()
    ).await?;
    let schema = serde_json::to_value(schemars::schema_for!(NetworkRelationshipResponse))
        .expect("Failed to serialize NetworkRelationshipResponse schema");
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

    sleep(Duration::from_secs(2)).await;

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

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<NetworkRelationship>,
    right: Arc<NetworkRelationship>,
) -> Result<String, Errors> {
    let left_normal_meta_context = left.apply(
        Arc::clone(&normalization_context),
    )?;
    let left_document = Document::from_normal_meta_context(
        &left_normal_meta_context,
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

    let right_normal_meta_context = right.apply(
        Arc::clone(&normalization_context),
    )?;
    let right_document = Document::from_normal_meta_context(
        &right_normal_meta_context,
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

    let mut basis_networks: Vec<Arc<BasisNetwork>> = Vec::new();
    left.collect_basis_networks(&mut basis_networks);
    right.collect_basis_networks(&mut basis_networks);

    let basis_network_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_network_contexts.as_ref().unwrap().clone()
    };

    let contexts: Vec<Arc<Context>> = basis_networks
        .iter()
        .flat_map(|basis_network| {
            basis_network_contexts
                .get(&basis_network.id)
                .unwrap()
                .iter()
                .take(3)
                .cloned()
        })
        .collect();

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string())
            })?
            .clone()
    };

    let context_strings: Vec<String> = contexts
        .iter()
        .map(|context| context.generate_context_string_network_relationship(Arc::clone(&normalization_context)))
        .collect::<Result<Vec<String>, Errors>>()?;
    let merged_samples = context_strings.join("\n\n---SNIPPET SEPARATOR---\n\n");

    Ok(format!(r##"
[SOURCE DOCUMENT EXAMPLES]
{}

[LEFT TARGET DOCUMENT]
{}

[RIGHT TARGET DOCUMENT]
{}
"##, merged_samples, left_document.to_string(), right_document.to_string()))
}
