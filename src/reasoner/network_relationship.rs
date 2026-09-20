use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::basis_graph::NetworkRelationship;
use crate::basis_network::BasisNetwork;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::graph_node::GraphNode;
use crate::prelude::*;
use crate::reasoner::{Capability, CompletionMetadata, Reasoner, ReasonerMetadata};

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    if left.id == right.id {
        network_relationship_reflexive(reasoner, Arc::clone(&normalization_context), left.clone())
            .await
    } else {
        network_relationship_comparative(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone(),
        )
        .await
    }
}

async fn network_relationship_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    network: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    let user_prompt = get_user_prompt_reflexive(
        reasoner,
        Arc::clone(&normalization_context),
        network.clone(),
    )?;

    log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
    log::debug!("{}", user_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");

    unimplemented!()
}

async fn network_relationship_comparative<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    unimplemented!()
}

fn get_user_prompt_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    network: Arc<BasisNetwork>,
) -> Result<String, Errors> {
    let parent = Arc::new(RwLock::new(GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        description: "dummy_parent".to_string(),
        hash: Hash::new(),
        subgraph_hash: Hash::new(),
        lineage: Lineage::new(),
        children: Vec::new(),
    }));

    let normal_meta_context =
        network.apply(Arc::clone(&normalization_context), Arc::clone(&parent))?;

    let format = DocumentFormat {
        format_type: DocumentType::Json,
        encoding: None,
        indent: Some(2),
        line_ending: None,
        headers: None,
        wrap_text: None,
        exclude_nulls: None,
        custom_delimiter: None,
    };

    let document = Document::from_normal_meta_context(&normal_meta_context, &format)?;

    let truncated = if document.data.len() > 3089 {
        format!(
            "{}\n...",
            document.data.chars().take(3086).collect::<String>()
        )
    } else {
        document.data.to_string()
    };

    log::debug!("truncated: {}", truncated);

    unimplemented!()
}
