use futures::future::try_join_all;
use std::collections::{HashSet, HashMap};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;
use async_recursion::async_recursion;

use crate::basis_field::BasisField;
use crate::basis_group::BasisGroup;
use crate::basis_node::BasisNode;
use crate::config::CONFIG;
use crate::llm::{LLM};
use crate::normalization_context::NormalizationContext;
use crate::translation_context::TranslationContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::context::Context;
use crate::translation_node::TranslationNode;
use crate::group_analysis::resolve_context_groups;

pub async fn get_translation_nodes<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    translation_context: Arc<RwLock<TranslationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<TranslationNodeID, Arc<TranslationNode>>, Errors> {
    log::trace!("In get_translation_nodes");




    let target_contexts = read_lock!(translation_context).must_get_unique_target_contexts()?;

    let mut unique_target_contexts: Vec<Arc<Context>> = Vec::new();
    let mut seen: HashSet<Lineage> = HashSet::new();
    for ctx in target_contexts {
        if seen.insert(ctx.lineage.clone()) {
            unique_target_contexts.push(ctx);
        }
    }






    let input_contexts = read_lock!(translation_context).must_get_unique_input_contexts()?;

    let mut unique_input_contexts: Vec<Arc<Context>> = Vec::new();
    let mut seen: HashSet<Lineage> = HashSet::new();
    for ctx in input_contexts {
        if seen.insert(ctx.lineage.clone()) {
            unique_input_contexts.push(ctx);
        }
    }




    let context_pairs: Vec<(Arc<Context>, Arc<Context>)> = unique_input_contexts.iter()
        .flat_map(|context_a| unique_target_contexts.iter().map(move |context_b| {
            (context_a.clone(), context_b.clone())
        }))
        .collect();


    log::info!("Number of context pairs: {}", context_pairs.len());








    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();





    for pair in context_pairs {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_translation_context = Arc::clone(&translation_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;

            let maybe_translation_node = get_translation_node(
                cloned_provider,
                cloned_reasoner,
                cloned_translation_context,
                pair,
                &cloned_options,
                &cloned_stage_context,
            )
            .await?;

            Ok(maybe_translation_node)
        });
        handles.push(handle);
    }

    let results: Vec<Result<Option<TranslationNode>, Errors>> = try_join_all(handles).await?;
    
    let translation_nodes: Vec<TranslationNode> = results.into_iter()
        .filter_map(|res| {
            match res {
                Ok(Some(translation_node)) => Some(Ok(translation_node)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<TranslationNode>, Errors>>()?;

    let hashmap: HashMap<ID, Arc<TranslationNode>> = translation_nodes.into_iter()
        .map(|translation_node| {
            let translation_node = Arc::new(translation_node);
            let id = translation_node.id.clone();
            (id, translation_node)
        })
        .collect();

    Ok(hashmap)
}

async fn get_translation_node<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    translation_context: Arc<RwLock<TranslationContext>>,
    context_pair: (Arc<Context>, Arc<Context>),
    options: &Options,
    stage_context: &StageContext,
) -> Result<Option<TranslationNode>, Errors> {
    let (input_context, target_context) = context_pair;

    if !options.regenerate {
        if let Some(maybe_translation_node) = provider.get_translation_node_by_lineages(
            &input_context.lineage,
            &target_context.lineage,
        ).await? {
            return Ok(maybe_translation_node);
        }
    }

    let (transformations, (tokens,)) = LLM::get_node_translation(
        Arc::clone(&translation_context),
        Arc::clone(&input_context),
        Arc::clone(&target_context)
    ).await?;

    if transformations.is_empty() {
        provider.save_translation_node(
            (input_context.lineage.clone(), target_context.lineage.clone()),
            None
        ).await?;

        Ok(None)
    } else {
        let translation_node = TranslationNode {
            id: ID::new(),
            source_lineage: input_context.lineage.clone(),
            target_lineage: target_context.lineage.clone(),
            transformations: transformations.clone(),
        };

        provider.save_translation_node(
            (input_context.lineage.clone(), target_context.lineage.clone()),
            Some(translation_node.clone())
        ).await?;

        Ok(Some(translation_node))
    }
}

pub async fn generate_basis_nodes<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In generate_basis_nodes");

    let basis_groups = {
        let lock = read_lock!(normalization_context);
        lock.basis_groups
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis groups not provided in meta context".to_string())
            })?
    };
    let (context_groups, _context_to_group) = resolve_context_groups(
        Arc::clone(&normalization_context)
    )?;

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
                context_group,
                &cloned_options,
                &cloned_stage_context,
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

async fn generate_basis_node<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    basis_group: Arc<BasisGroup>,
    context_group: Vec<Arc<Context>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<BasisNode, Errors> {
    log::trace!("In generate_basis_node");

    stage_context.record_events("Node analysis", 0);

    let basis_lineage: BasisLineage = basis_group.get_basis_lineage();

    if !options.regenerate {
        if let Some(basis_node) = provider.get_basis_node_by_lineage(&basis_lineage).await? {
            return Ok(basis_node);
        }
    }

    let (basis_node, metadata) = reasoner.basis_node(
        Arc::clone(&normalization_context),
        basis_group,
        context_group,
    ).await?;

    stage_context.record_events("Node analysis", metadata.tokens.into());

    provider
        .save_basis_node(&basis_lineage, basis_node.clone())
        .await?;
    
    Ok(basis_node)
}
