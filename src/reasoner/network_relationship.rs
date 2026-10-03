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
use crate::normal_context::NormalContext;

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

    let normal_contexts: Vec<Arc<NormalContext>> = {
        let root = read_lock!(normal_meta_context.graph_root);
        root.children
            .iter()
            .take(5)
            .map(|child| {
                let id = read_lock!(child).id.clone();
                normal_meta_context
                    .contexts_lookup
                    .get(&id)
                    .cloned()
                    .ok_or(Errors::DeficientNormalizationContextError(format!(
                        "No normal context found for graph node {}",
                        id.to_string()
                    )))
            })
            .collect::<Result<Vec<_>, Errors>>()?
    };

    let context_string = normal_contexts
        .iter()
        .try_fold(String::new(), |acc, normal_context| {
            let context_string = Context::generate_context_string_network_relationship(
                Arc::clone(&normalization_context),
                normal_context.contexts.clone()
            )?;

            Ok::<String, Errors>(if acc.is_empty() {
                context_string
            } else {
                format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string)
            })
        })?;

    Ok(context_string)
}
