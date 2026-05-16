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

pub async fn get_context_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<ID, Vec<Arc<Context>>>, Errors> {
    log::trace!("In get_context_groups");

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

    let basis_groups = {
        let lock = read_lock!(meta_context);
        lock.basis_groups
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Basis groups not provided in meta context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let mut groups_by_acyclic: HashMap<Lineage, Vec<Arc<BasisGroup>>> = HashMap::new();
    for group in basis_groups {
        groups_by_acyclic
            .entry(group.acyclic_lineage.clone())
            .or_insert_with(Vec::new)
            .push(group);
    }

    let mut context_groups: HashMap<ID, Vec<Arc<Context>>> = HashMap::new();

    for context in unique_contexts {
        let Some(candidate_groups) = groups_by_acyclic.get(&context.acyclic_lineage) else {
            continue;
        };

        let mut indexed_lineages: HashSet<Lineage> = HashSet::new();
        let mut next_depth = 0;
        let mut indexed_exhausted = false;

        for group in candidate_groups {
            let matches = match (&group.lineage, &group.indexed_lineage) {
                (None, _) => true,
                (Some(l), None) => context.lineage == *l,
                (Some(l), Some(il)) => {
                    if context.lineage != *l {
                        false
                    } else if indexed_lineages.contains(il) {
                        true
                    } else if indexed_exhausted {
                        false
                    } else {
                        let mut found = false;
                        while let Some(new_il) = context.get_indexed_lineage(next_depth) {
                            next_depth += 1;
                            let is_match = new_il == *il;
                            indexed_lineages.insert(new_il);
                            if is_match {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            indexed_exhausted = true;
                        }
                        found
                    }
                }
            };

            if matches {
                context_groups
                    .entry(group.id.clone())
                    .or_insert_with(Vec::new)
                    .push(Arc::clone(&context));
            }
        }
    }

    Ok(context_groups)
}

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
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_acyclic_basis_groups(
                cloned_provider,
                cloned_meta_context,
                acyclic_lineage,
                contexts_in_group,
                cloned_options,
                cloned_stage_context,
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
    options: Options,
    stage_context: StageContext,
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_acyclic_basis_groups");

    if !options.regenerate {
        let cached: Vec<BasisGroup> = provider
            .get_basis_groups_by_acyclic_lineage(&acyclic_lineage).await?
            .into_iter()
            .collect();
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if contexts_in_group.len() == 1 {
        log::info!("Acyclic context group has only one item.");
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: None,
            indexed_lineage: None,
        })
    } else {
        let (is_match, (tokens,)) = LLM::infer_group_match(
            Arc::clone(&meta_context),
            contexts_in_group.clone(),
            30
        ).await?;

        if is_match {
            log::info!("Contexts with acyclic lineage: {} have been inferred to match",
                acyclic_lineage.to_string());
            stage_context.record_events("Group analysis", tokens);
            Some(BasisGroup {
                id: ID::new(),
                acyclic_lineage: acyclic_lineage.clone(),
                lineage: None,
                indexed_lineage: None,
            })
        } else {
            None
        }
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(&acyclic_lineage, None, None, basis_group.clone()).await?;
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
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_cyclic_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                lineage,
                contexts_in_subgroup,
                cloned_options,
                cloned_stage_context
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
    contexts_in_group: Vec<Arc<Context>>,
    options: Options,
    stage_context: StageContext
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_cyclic_basis_groups");

    if !options.regenerate {
        let cached: Vec<BasisGroup> = provider
            .get_basis_groups_by_lineage(&acyclic_lineage, &lineage).await?
            .into_iter()
            .collect();
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if contexts_in_group.len() == 1 {
        log::info!("Cyclic context group has only one item.");
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: None,
        })
    } else {
        let (is_match, (tokens,)) = LLM::infer_group_match(
            Arc::clone(&meta_context),
            contexts_in_group.clone(),
            20
        ).await?;

        if is_match {
            log::info!("Contexts with cyclic lineage: {} have been inferred to match", lineage.to_string());
            stage_context.record_events("Group analysis", tokens);
            Some(BasisGroup {
                id: ID::new(),
                acyclic_lineage: acyclic_lineage.clone(),
                lineage: Some(lineage.clone()),
                indexed_lineage: None,
            })
        } else {
            None
        }
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(&acyclic_lineage, Some(&lineage), None, basis_group.clone()).await?;
        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with cyclic lineage: {} have been inferred to not match", lineage.to_string());

    let mut next_depth = 0;
    let mut indexed_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
    let mut unique_contexts: Vec<Arc<Context>> = Vec::new();

    loop {
        indexed_contexts.clear();
        unique_contexts.clear();

        for context in &contexts_in_group {
            if let Some(indexed_lineage) = context.get_indexed_lineage(next_depth) {
                indexed_contexts
                    .entry(indexed_lineage.clone())
                    .or_insert_with(Vec::new)
                    .push(context.clone());
            } else {
                log::warn!("Ran out of indexed lineages");
                unique_contexts.push(context.clone());
            }
        }

        if indexed_contexts.len() > 1 {
            break;
        }

        if unique_contexts.len() == contexts_in_group.len() {
            // TODO: do something with unique contexts
            return Ok(Vec::new());
        }

        next_depth += 1;
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
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_indexed_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                indexed_lineage.clone(),
                contexts_in_subgroup.clone(),
                0,
                cloned_options,
                cloned_stage_context
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
    depth: usize,
    options: Options,
    stage_context: StageContext,
) -> Result<Vec<BasisGroup>, Errors> {
    log::trace!("In get_index_basis_groups");

    if !options.regenerate {
        let cached = provider
            .get_basis_groups_by_indexed_lineage(&acyclic_lineage, &lineage, &indexed_lineage).await?;
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if contexts_in_group.len() == 1 {
        log::info!("Indexed context group has only one item.");
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: Some(indexed_lineage.clone()),
        })
    } else {
        let (is_match, (tokens,)) = LLM::infer_group_match(
            Arc::clone(&meta_context),
            contexts_in_group.clone(),
            10
        ).await?;

        if is_match {
            log::info!("Contexts with indexed lineage: {} have been inferred to match", indexed_lineage.to_string());
            stage_context.record_events("Group analysis", tokens);
            Some(BasisGroup {
                id: ID::new(),
                acyclic_lineage: acyclic_lineage.clone(),
                lineage: Some(lineage.clone()),
                indexed_lineage: Some(indexed_lineage.clone()),
            })
        } else {
            None
        }
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(&acyclic_lineage, Some(&lineage), Some(&indexed_lineage), basis_group.clone()).await?;
        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with indexed lineage: {} have been inferred to not match", indexed_lineage.to_string());

    let mut next_depth = depth + 1;
    let mut indexed_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
    let mut unique_contexts: Vec<Arc<Context>> = Vec::new();

    loop {
        indexed_contexts.clear();
        unique_contexts.clear();

        for context in &contexts_in_group {
            if let Some(indexed_lineage) = context.get_indexed_lineage(next_depth) {
                indexed_contexts
                    .entry(indexed_lineage.clone())
                    .or_insert_with(Vec::new)
                    .push(context.clone());
            } else {
                log::warn!("Ran out of indexed lineages");
                unique_contexts.push(context.clone());
            }
        }

        if indexed_contexts.len() > 1 {
            break;
        }

        if unique_contexts.len() == contexts_in_group.len() {
            // TODO: do something with unique contexts
            return Ok(Vec::new());
        }

        next_depth += 1;
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
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_indexed_basis_groups(
                cloned_provider,
                cloned_meta_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                next_indexed_lineage.clone(),
                contexts_in_subgroup.clone(),
                next_depth,
                cloned_options,
                cloned_stage_context,
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

    let document_summary = Arc::new(get_original_document_condensed(Arc::clone(&meta_context))?);
    let basis_groups = {
        let lock = read_lock!(meta_context);
        lock.basis_groups
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Basis groups not provided in meta context".to_string())
            })?
    };
    let context_groups = get_context_groups(Arc::clone(&provider), Arc::clone(&meta_context)).await?;
    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (basis_group_id, group) in context_groups {
        stage_context.record_events("Node analysis", 0);

        let basis_group = basis_groups.get(&basis_group_id).unwrap();
        let basis_lineage = basis_group.get_basis_lineage();

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
                basis_lineage,
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
    basis_lineage: Lineage,
) -> Result<BasisNode, Errors> {
    log::trace!("In get_basis_node");

    let first = group.first().unwrap();

    if first.data_node.fields.is_empty() {
        return Err(Errors::InsufficientPrerequisites(
            "get_basis_node called with empty fields group".to_string(),
        ));
    }

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
