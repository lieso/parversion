use futures::future::try_join_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::basis_node::BasisNode;
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::{LLM, NodeGroupClassification};
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::traversal::{get_original_document_condensed};
use crate::context::Context;

pub async fn get_basis_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisGroup>>, Errors> {
    log::trace!("In get_basis_groups");

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Contexts not provided in meta context".to_string())
            })?
    };
    
    log::info!("Total number of contexts: {}", contexts.len());

    let non_empty_contexts = contexts
        .values()
        .filter(|context| !context.data_node.fields.is_empty())
        .collect();

    log::info!("Number of non-empty contexts: {}", non_empty_contexts.len());

    let mut acyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in non_empty_contexts {
        acyclic_contexts
            .entry(context.acyclic_lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }
    
    log::info!("Number of acyclic contexts: {}", acyclic_contexts.len());

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));



    unimplemented!();
}

pub async fn get_basis_nodes<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In get_basis_nodes");

    let document_summary = Arc::new(get_original_document_condensed(Arc::clone(&meta_context))?);
    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for group in context_groups {
        stage_context.record_events("Node analysis", 0);

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();
        let cloned_document_summary = Arc::clone(&document_summary);

        let handle = task::spawn(async move {
            let _permit = permit;
            let basis_node = get_basis_node(
                cloned_provider,
                cloned_meta_context,
                group,
                &cloned_options,
                &cloned_stage_context,
                &cloned_document_summary,
            )
            .await?;

            Ok((basis_node.id.clone(), Arc::new(basis_node)))
        });
        handles.push(handle);
    }

    let results: Vec<Result<(ID, Arc<BasisNode>), Errors>> = try_join_all(handles).await?;

    let hashmap_results: HashMap<ID, Arc<BasisNode>> =
        results.into_iter().collect::<Result<_, _>>()?;

    Ok(hashmap_results)
}

async fn get_basis_node<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    group: Vec<Arc<Context>>,
    options: &Options,
    stage_context: &StageContext,
    document_summary: &str,
) -> Result<BasisNode, Errors> {
    log::trace!("In get_basis_node");

    let first = group.first().unwrap();

    if first.data_node.fields.is_empty() {
        return Err(Errors::InsufficientPrerequisites(
            "get_basis_node called with empty fields group".to_string(),
        ));
    }

    let basis_lineage = first.basis_lineage().ok_or_else(|| {
        Errors::InsufficientPrerequisites("basis_lineage not set on context".to_string())
    })?;

    let data_node = &first.data_node;
    let hash = data_node.hash.clone();
    let description = data_node.description.clone();

    if !options.regenerate {
        if let Some(basis_node) = provider.get_basis_node_by_lineage(&basis_lineage).await? {
            return Ok(basis_node);
        };
    }

    let (field_transformations, (tokens,)) = LLM::get_node_transformations(
        group,
        Arc::clone(&meta_context),
        document_summary,
    ).await?;

    let basis_node = BasisNode {
        id: ID::new(),
        hash,
        description,
        lineage: basis_lineage.clone(),
        transformations: field_transformations,
    };

    provider
        .save_basis_node(&basis_lineage, basis_node.clone())
        .await?;

    stage_context.record_events("Node analysis", tokens);

    Ok(basis_node)
}
