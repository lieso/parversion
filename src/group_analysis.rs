use futures::future::try_join_all;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use tokio::task;
use async_recursion::async_recursion;

use crate::prelude::*;
use crate::basis_field::BasisField;
use crate::basis_group::{BasisGroup, BasisGroupMetadata};

pub async fn get_basis_groups<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisGroup>>, Errors> {
    log::trace!("In get_basis_groups");

    let non_empty_contexts = get_non_empty_contexts(Arc::clone(&normalization_context))?;
    log::info!("Number of non-empty contexts: {}", non_empty_contexts.len());

    let mut acyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
    for context in non_empty_contexts {
        acyclic_contexts
            .entry(context.acyclic_lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }
    log::info!("Number of acyclic contexts: {}", acyclic_contexts.len());

    let mut handles = Vec::new();

    for (acyclic_lineage, candidate_group) in acyclic_contexts {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();

        let handle = task::spawn(async move {
            get_acyclic_basis_groups(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                acyclic_lineage,
                candidate_group,
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

async fn get_acyclic_basis_groups<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    acyclic_lineage: Lineage,
    candidate_group: Vec<Arc<Context>>,
    options: Options,
    stage_context: StageContext,
) -> Result<Vec<BasisGroup>, Errors> {
    stage_context.record_events("Group analysis", 0);

    if !options.regenerate {
        let basis_groups: Vec<BasisGroup> = provider
            .get_basis_groups_by_acyclic_lineage(&acyclic_lineage).await?
            .into_iter()
            .collect();
        if !basis_groups.is_empty() {
            return Ok(basis_groups);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if candidate_group.len() == 1 {
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: None,
            indexed_lineage: None,
            metadata: BasisGroupMetadata {
                prompt_hash: None,
            }
        })
    } else {
        let (maybe_basis_group, metadata) = reasoner.basis_group(
            Arc::clone(&normalization_context),
            candidate_group.clone(),
            acyclic_lineage.clone(),
            None,
            None,
        ).await?;

        stage_context.record_events("Group analysis", metadata.tokens.into());

        maybe_basis_group
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(&acyclic_lineage, None, None, basis_group.clone()).await?;
        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with acyclic lineage: {} have been inferred to not match. Proceeding to subgroup by lineage", acyclic_lineage.to_string());

    let mut cyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
    for context in candidate_group {
        cyclic_contexts
            .entry(context.lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }
    log::info!("Number of cyclic contexts in candidate group: {}", cyclic_contexts.len());

    let mut handles = Vec::new();

    for (lineage, candidate_subgroup) in cyclic_contexts {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();
        let cloned_acyclic_lineage = acyclic_lineage.clone();

        let handle = task::spawn(async move {
            get_cyclic_basis_groups(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                cloned_acyclic_lineage,
                lineage,
                candidate_subgroup,
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

async fn get_cyclic_basis_groups<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    acyclic_lineage: Lineage,
    lineage: Lineage,
    candidate_group: Vec<Arc<Context>>,
    options: Options,
    stage_context: StageContext
) -> Result<Vec<BasisGroup>, Errors> {
    stage_context.record_events("Group analysis", 0);

    if !options.regenerate {
        let cached: Vec<BasisGroup> = provider
            .get_basis_groups_by_lineage(&acyclic_lineage, &lineage).await?
            .into_iter()
            .collect();
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if candidate_group.len() == 1 {
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: None,
            metadata: BasisGroupMetadata {
                prompt_hash: None,
            }
        })
    } else {
        let (maybe_basis_group, metadata) = reasoner.basis_group(
            Arc::clone(&normalization_context),
            candidate_group.clone(),
            acyclic_lineage.clone(),
            Some(lineage.clone()),
            None,
        ).await?;

        stage_context.record_events("Group analysis", metadata.tokens.into());

        maybe_basis_group
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(
            &acyclic_lineage,
            Some(&lineage),
            None,
            basis_group.clone()
        ).await?;
        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with cyclic lineage: {} have been inferred to not match. Proceeding to recursively subgroup by indexed lineage", lineage.to_string());

    let (indexed_contexts, singular_contexts, depth) = collect_indexed_subgroups(
        candidate_group.clone(),
        0
    );

    if singular_contexts.len() == candidate_group.len() {
        unimplemented!();
    }

    let mut handles = Vec::new();

    for (indexed_lineage, candidate_subgroup) in indexed_contexts {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();
        let cloned_acyclic_lineage = acyclic_lineage.clone();
        let cloned_lineage = lineage.clone();

        let handle = task::spawn(async move {
            get_indexed_basis_groups(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                indexed_lineage.clone(),
                candidate_subgroup,
                depth,
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
async fn get_indexed_basis_groups<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    acyclic_lineage: Lineage,
    lineage: Lineage,
    indexed_lineage: Lineage,
    candidate_group: Vec<Arc<Context>>,
    depth: usize,
    options: Options,
    stage_context: StageContext,
) -> Result<Vec<BasisGroup>, Errors> {
    stage_context.record_events("Group analysis", 0);

    if !options.regenerate {
        let cached = provider
            .get_basis_groups_by_indexed_lineage(&acyclic_lineage, &lineage, &indexed_lineage).await?;
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let maybe_basis_group: Option<BasisGroup> = if candidate_group.len() == 1 {
        Some(BasisGroup {
            id: ID::new(),
            acyclic_lineage: acyclic_lineage.clone(),
            lineage: Some(lineage.clone()),
            indexed_lineage: Some(indexed_lineage.clone()),
            metadata: BasisGroupMetadata {
                prompt_hash: None,
            }
        })
    } else {
        let (maybe_basis_group, metadata) = reasoner.basis_group(
            Arc::clone(&normalization_context),
            candidate_group.clone(),
            acyclic_lineage.clone(),
            Some(lineage.clone()),
            Some(indexed_lineage.clone())
        ).await?;

        stage_context.record_events("Group analysis", metadata.tokens.into());

        maybe_basis_group
    };

    if let Some(basis_group) = maybe_basis_group {
        provider.save_basis_group(
            &acyclic_lineage,
            Some(&lineage),
            Some(&indexed_lineage),
            basis_group.clone()
        ).await?;
        return Ok(vec![basis_group]);
    }

    log::info!("Contexts with indexed lineage: {} have been inferred to not match. Proceeding to increase depth", indexed_lineage.to_string());

    let (indexed_contexts, singular_contexts, next_depth) = collect_indexed_subgroups(
        candidate_group.clone(),
        depth + 1,
    );

    if singular_contexts.len() == candidate_group.len() {
        unimplemented!();
    }

    let mut handles = Vec::new();

    for (next_indexed_lineage, candidate_subgroup) in indexed_contexts {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_acyclic_lineage = acyclic_lineage.clone();
        let cloned_lineage = lineage.clone();
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            get_indexed_basis_groups(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                cloned_acyclic_lineage,
                cloned_lineage,
                next_indexed_lineage,
                candidate_subgroup,
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

fn collect_indexed_subgroups(
    candidate_group: Vec<Arc<Context>>,
    start_depth: usize
) -> (
    HashMap<Lineage, Vec<Arc<Context>>>, 
    Vec<Arc<Context>>, // singular contexts
    usize // depth
) {
    let mut next_depth = start_depth;
    let mut indexed_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
    let mut singular_contexts: Vec<Arc<Context>> = Vec::new();

    loop {
        indexed_contexts.clear();
        singular_contexts.clear();

        for context in &candidate_group {
            if let Some(indexed_lineage) = context.get_indexed_lineage(next_depth) {
                indexed_contexts
                    .entry(indexed_lineage.clone())
                    .or_insert_with(Vec::new)
                    .push(context.clone());
            } else {
                singular_contexts.push(context.clone());
            }
        }

        if indexed_contexts.len() > 1 {
            break;
        }

        if singular_contexts.len() == candidate_group.len() {
            break;
        }

        next_depth += 1;
    }

    (indexed_contexts, singular_contexts, next_depth)
}

fn get_non_empty_contexts(normalization_context: Arc<RwLock<NormalizationContext>>) -> Result<Vec<Arc<Context>>, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Contexts not provided in normalization context".to_string())
            })?
    };

    let basis_fields: Vec<Arc<BasisField>> = {
        let lock = read_lock!(normalization_context);
        lock.basis_fields
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis fields not provided in normalization context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let contexts: Vec<Arc<Context>> = meta_context.contexts.values().cloned().collect();

    log::info!("Number of contexts: {}", contexts.len());

    let non_empty_contexts: Vec<Arc<Context>> = contexts
        .into_iter()
        .filter(|context| {
            for (field, _value) in &context.data_node.fields {
                for basis_field in &basis_fields {
                    if basis_field.name == *field {
                        return true;
                    }
                }
            }

            false
        })
        .collect();

    log::info!("Number of non-empty contexts: {}", non_empty_contexts.len());

    Ok(non_empty_contexts)
}
