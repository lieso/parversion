use futures::future::try_join_all;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::basis_group::BasisGroup;
use crate::basis_node::BasisNode;
use crate::config::CONFIG;
use crate::context::Context;
use crate::group_analysis::resolve_context_groups;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::translation_context::TranslationContext;

pub async fn generate_basis_nodes<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<
    (
        HashMap<ID, Arc<BasisNode>>,
        HashMap<ID, Vec<Arc<Context>>>,
        HashMap<ID, Arc<BasisNode>>,
    ),
    Errors,
> {
    log::trace!("In generate_basis_nodes");

    let basis_groups = {
        let lock = read_lock!(normalization_context);
        lock.basis_groups.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis groups not provided in meta context".to_string(),
            )
        })?
    };
    let (context_groups, _context_to_group) =
        resolve_context_groups(Arc::clone(&normalization_context))?;

    log::info!("Number of groups: {}", context_groups.len());

    let mut handles = Vec::new();

    for (basis_group_id, context_group) in context_groups {
        let basis_group = basis_groups.get(&basis_group_id).unwrap().clone();
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();

        let handle = task::spawn(async move {
            let basis_node = generate_basis_node(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                basis_group.clone(),
                context_group.clone(),
                &cloned_options,
                &cloned_stage_context,
            )
            .await?;

            Ok::<_, Errors>((
                basis_node.id.clone(),
                Arc::new(basis_node),
                context_group.clone(),
            ))
        });

        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    let mut basis_nodes = HashMap::new();
    let mut basis_node_to_context_group = HashMap::new();
    let mut context_to_basis_node = HashMap::new();

    for result in results {
        let (id, basis_node, context_group) = result?;
        basis_nodes.insert(id.clone(), basis_node.clone());
        basis_node_to_context_group.insert(id, context_group.clone());

        for context in context_group {
            context_to_basis_node.insert(context.id.clone(), basis_node.clone());
        }
    }

    Ok((
        basis_nodes,
        basis_node_to_context_group,
        context_to_basis_node,
    ))
}

async fn generate_basis_node<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_group: Arc<BasisGroup>,
    context_group: Vec<Arc<Context>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<BasisNode, Errors> {
    stage_context.record_events("Node analysis", 0);

    let basis_lineage: BasisLineage = basis_group.get_basis_lineage();

    if !options.regenerate {
        if let Some(basis_node) = provider.get_basis_node_by_lineage(&basis_lineage).await? {
            return Ok(basis_node);
        }
    }

    let (basis_node, metadata) = reasoner
        .basis_node(
            Arc::clone(&normalization_context),
            basis_group,
            context_group,
        )
        .await?;

    stage_context.record_events("Node analysis", metadata.tokens.into());

    provider
        .save_basis_node(&basis_lineage, basis_node.clone())
        .await?;

    Ok(basis_node)
}
