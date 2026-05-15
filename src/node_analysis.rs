use futures::future::try_join_all;
use std::collections::{HashSet, HashMap};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;
use async_recursion::async_recursion;

use crate::basis_group::BasisGroup;
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

    // NOTE: there are three duplicate contexts: one for each data node, graph node and document node

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Contexts not provided in meta context".to_string())
            })?
    };

    log::info!("Number of contexts: {}", contexts.len());

    let non_empty_contexts: Vec<Arc<Context>> = contexts
        .into_values()
        .filter(|context| !context.data_node.fields.is_empty())
        .collect();

    log::info!("Number of non-empty contexts: {}", non_empty_contexts.len());

    let mut seen = HashSet::new();
    let unique_contexts: Vec<Arc<Context>> = non_empty_contexts
        .into_iter()
        .filter(|context| seen.insert(context.id.clone()))
        .collect();

    log::info!("Number of unique contexts: {}", unique_contexts.len());

    let mut acyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in unique_contexts {
        acyclic_contexts
            .entry(context.acyclic_lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }

    log::info!("Number of acyclic contexts: {}", acyclic_contexts.len());

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (acyclic_lineage, contexts_in_group) in acyclic_contexts {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);

        let handle = task::spawn(async move {
            let _permit = permit;
            get_acyclic_basis_groups(
                cloned_provider,
                cloned_meta_context,
                acyclic_lineage,
                contexts_in_group,
            )
            .await
        });
        handles.push(handle);
    }

    let results: Vec<Result<Vec<BasisGroup>, Errors>> = try_join_all(handles).await?;

    let flattened: Vec<BasisGroup> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    let basis_groups: HashMap<ID, Arc<BasisGroup>> = flattened
        .into_iter()
        .map(|basis_group| (basis_group.id.clone(), Arc::new(basis_group)))
        .collect();

    Ok(basis_groups)
}

async fn get_acyclic_basis_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    acyclic_lineage: Lineage,
    contexts_in_group: Vec<Arc<Context>>,
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_acyclic_basis_groups");

    let (is_match, is_meaningful, (tokens,)) = LLM::infer_group_match(
        Arc::clone(&meta_context),
        contexts_in_group.clone()
    ).await?; 

    if is_match {
        log::info!("Contexts with acyclic lineage: {} have been inferred to match", acyclic_lineage.to_string());

        let basis_group = BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: None,
            indexed_lineage: None,
            is_meaningful,
        };

        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with acyclic lineage: {} have been inferred to not match", acyclic_lineage.to_string());

    let mut cyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in contexts_in_group {
        cyclic_contexts
            .entry(context.lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }

    log::info!("Number of cyclic contexts: {}", cyclic_contexts.len());

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (lineage, contexts_in_subgroup) in cyclic_contexts {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_acyclic_lineage = acyclic_lineage.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_cyclic_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                lineage,
                contexts_in_subgroup
            )
            .await
        });
        handles.push(handle);
    }

    let results: Vec<Result<Vec<BasisGroup>, Errors>> = try_join_all(handles).await?;

    let flattened: Vec<BasisGroup> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(flattened)
}

async fn get_cyclic_basis_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    acyclic_lineage: Lineage,
    lineage: Lineage,
    contexts_in_group: Vec<Arc<Context>>
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_cyclic_basis_groups");

    let (is_match, is_meaningful, (tokens,)) = LLM::infer_group_match(
        Arc::clone(&meta_context),
        contexts_in_group.clone()
    ).await?;

    if is_match {
        log::info!("Contexts with cyclic lineage: {} have been inferred to match", lineage.to_string());

        let basis_group = BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: None,
            is_meaningful,
        };

        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with cyclic lineage: {} have been inferred to not match", lineage.to_string());

    let mut indexed_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in contexts_in_group {
        if let Some(indexed_lineage) = context.get_indexed_lineage(1) {
            indexed_contexts
                .entry(indexed_lineage.clone())
                .or_insert_with(Vec::new)
                .push(context);
        } else {
            unimplemented!();
        }
    }

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (indexed_lineage, contexts_in_subgroup) in indexed_contexts {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_acyclic_lineage = acyclic_lineage.clone();
        let cloned_lineage = lineage.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_indexed_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                indexed_lineage.clone(),
                contexts_in_subgroup.clone(),
                1
            )
            .await
        });
        handles.push(handle);
    }

    let results: Vec<Result<Vec<BasisGroup>, Errors>> = try_join_all(handles).await?;

    let flattened: Vec<BasisGroup> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(flattened)
}

#[async_recursion]
async fn get_indexed_basis_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    acyclic_lineage: Lineage,
    lineage: Lineage,
    indexed_lineage: Lineage,
    contexts_in_group: Vec<Arc<Context>>,
    depth: usize
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_index_basis_groups");

    let (is_match, is_meaningful, (tokens,)) = LLM::infer_group_match(
        Arc::clone(&meta_context),
        contexts_in_group.clone()
    ).await?;

    if is_match {
        log::info!("Contexts with indexed lineage: {} have been inferred to match", indexed_lineage.to_string());

        let basis_group = BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: Some(indexed_lineage),
            is_meaningful,
        };

        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with indexed lineage: {} have been inferred to not match", indexed_lineage.to_string());

    let mut indexed_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in contexts_in_group {
        if let Some(indexed_lineage) = context.get_indexed_lineage(depth + 1) {
            indexed_contexts
                .entry(indexed_lineage.clone())
                .or_insert_with(Vec::new)
                .push(context);
        } else {
            // If we've reached this point, that implies we've calculated indexed
            // lineages all the way up to the root node, but did not infer a match for some reason
            // That's fine. This context can just be a group of one, implying it's unique
            // across the entire graph and justly deserves individual analysis
            unimplemented!();
        }
    }

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (next_indexed_lineage, contexts_in_subgroup) in indexed_contexts {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_acyclic_lineage = acyclic_lineage.clone();
        let cloned_lineage = lineage.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_indexed_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                next_indexed_lineage.clone(),
                contexts_in_subgroup.clone(),
                depth + 1
            )
            .await
        });
        handles.push(handle);
    }

    let results: Vec<Result<Vec<BasisGroup>, Errors>> = try_join_all(handles).await?;

    let flattened: Vec<BasisGroup> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(flattened)
}

pub async fn get_basis_nodes<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In get_basis_nodes");
    
    unimplemented!();

    //let document_summary = Arc::new(get_original_document_condensed(Arc::clone(&meta_context))?);
    //let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    //let semaphore = Arc::new(Semaphore::new(max_concurrency));
    //let mut handles = Vec::new();

    //for group in context_groups {
    //    stage_context.record_events("Node analysis", 0);

    //    let permit = semaphore.clone().acquire_owned().await.unwrap();
    //    let cloned_provider = Arc::clone(&provider);
    //    let cloned_meta_context = Arc::clone(&meta_context);
    //    let cloned_options = options.clone();
    //    let cloned_stage_context = stage_context.clone();
    //    let cloned_document_summary = Arc::clone(&document_summary);

    //    let handle = task::spawn(async move {
    //        let _permit = permit;
    //        let basis_node = get_basis_node(
    //            cloned_provider,
    //            cloned_meta_context,
    //            group,
    //            &cloned_options,
    //            &cloned_stage_context,
    //            &cloned_document_summary,
    //        )
    //        .await?;

    //        Ok((basis_node.id.clone(), Arc::new(basis_node)))
    //    });
    //    handles.push(handle);
    //}

    //let results: Vec<Result<(ID, Arc<BasisNode>), Errors>> = try_join_all(handles).await?;

    //let hashmap_results: HashMap<ID, Arc<BasisNode>> =
    //    results.into_iter().collect::<Result<_, _>>()?;

    //Ok(hashmap_results)
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
