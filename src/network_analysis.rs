use futures::future::try_join_all;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::classification::Classification;
use crate::basis_network::{BasisNetwork, BasisNetworkMetadata};
use crate::basis_graph::BasisGraph;
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::LLM;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::translation_network::TranslationNetwork;
use crate::group_analysis::{resolve_context_groups};
use crate::basis_node::BasisNode;

pub async fn generate_basis_networks<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<(
    HashMap<BasisNetworkID, Arc<BasisNetwork>>,
    HashMap<BasisNetworkID, Vec<Arc<Context>>>,
    HashMap<ContextID, Arc<BasisNetwork>>
), Errors> {
    log::trace!("In generate_basis_networks");

    let candidate_networks = get_candidate_networks(Arc::clone(&normalization_context))?;
    log::info!("Number of candidate networks: {}", candidate_networks.len());

    let mut handles = Vec::new();

    for (basis_lineages_hash, (_basis_lineages, candidates)) in candidate_networks {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();

        let handle = task::spawn(async move {
            let basis_network = generate_basis_network(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                &cloned_options,
                &cloned_stage_context,
                basis_lineages_hash,
                candidates.clone(),
            )
            .await?;

            Ok((basis_network.id.clone(), Arc::new(basis_network), candidates.clone()))
        });

        handles.push(handle);
    }

    let results: Vec<(ID, Arc<BasisNetwork>, Vec<Arc<Context>>)> = try_join_all(handles).await?
        .into_iter()
        .collect::<Result<Vec<_>, Errors>>()?;

    let basis_networks: HashMap<ID, Arc<BasisNetwork>> = results
        .clone()
        .into_iter()
        .map(|(id, basis_network, _contexts)| (id, basis_network))
        .collect();

    let network_contexts: HashMap<BasisNetworkID, Vec<Arc<Context>>> = results
        .clone()
        .into_iter()
        .map(|(id, _basis_network, contexts)| (id, contexts))
        .collect();

    let context_networks: HashMap<ContextID, Arc<BasisNetwork>> = results
        .clone()
        .into_iter()
        .flat_map(|(_id, basis_network, contexts)| {
            contexts
                .into_iter()
                .map(move |context| (context.id.clone(), basis_network.clone()))
        })
        .collect();

    Ok((basis_networks, network_contexts, context_networks))
}

async fn generate_basis_network<R: Reasoner, P: Provider>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    basis_lineages_hash: Hash,
    context_group: Vec<Arc<Context>>
) -> Result<BasisNetwork, Errors> {
    stage_context.record_events("Network analysis", 0);

    if !options.regenerate {
        if let Some(basis_network) = provider.get_basis_network_by_basis_lineages(&basis_lineages_hash).await? {
            return Ok(basis_network);
        }
    }

    let (basis_network, metadata) = reasoner.basis_network(
        Arc::clone(&normalization_context),
        basis_lineages_hash,
        context_group
    ).await?;

    stage_context.record_events("Network analysis", metadata.tokens.into());

    provider
        .save_basis_network(basis_network.clone())
        .await?;

    Ok(basis_network)
}

fn get_candidate_networks(
    normalization_context: Arc<RwLock<NormalizationContext>>
) -> Result<HashMap<Hash, (HashSet<BasisLineage>, Vec<Arc<Context>>)>, Errors> {
    let graph_root = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone()
            .ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
            .graph_root.clone()
    };

    let mut candidate_networks: HashMap<Hash, (HashSet<BasisLineage>, Vec<Arc<Context>>)> = HashMap::new();

    fn recurse(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        candidate_networks: &mut HashMap<Hash, (HashSet<BasisLineage>, Vec<Arc<Context>>)>,
        graph: Graph,
    ) -> Result<HashSet<BasisLineage>, Errors> {
        let children = read_lock!(graph).children.clone();

        let mut set: HashSet<BasisLineage> = HashSet::new();

        for child in &children {
            let child_set = recurse(
                Arc::clone(&normalization_context),
                candidate_networks,
                Arc::clone(&child),
            )?;

            set.extend(child_set.iter().cloned());
        }

        let mut transformation_count: usize = set.len();

        if let Some(basis_node) = read_lock!(graph).resolve_basis_node(Arc::clone(&normalization_context))? {
            if !basis_node.transformations.is_empty() {
                set.insert(basis_node.lineage.clone());
                transformation_count += basis_node.transformations.len();
            }
        }

        if transformation_count > 1 {
            let basis_lineages: Hash = {
                let items: Vec<BasisLineage> = set.iter().cloned().collect();
                let mut hash = Hash::from_items(items);
                hash.sort();
                hash.finalize();
                hash
            };

            let context = {
                let meta_context = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
                };

                meta_context.contexts_lookup
                    .get(&read_lock!(graph).id)
                    .cloned()
                    .unwrap()
            };

            candidate_networks
                .entry(basis_lineages)
                .or_insert_with(|| (set.clone(), Vec::new()))
                .1
                .push(context.clone());

            return Ok(HashSet::new());
        }

        Ok(set)
    }

    recurse(
        Arc::clone(&normalization_context),
        &mut candidate_networks,
        Arc::clone(&graph_root),
    )?;

    Ok(candidate_networks)
}

pub async fn get_translation_networks<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    translation_context: Arc<RwLock<TranslationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<TranslationNetworkID, Arc<TranslationNetwork>>, Errors> {
    log::trace!("In get_translation_networks");

    let target_contexts = {
        let lock = read_lock!(translation_context);
        let meta_context = lock.target_meta_context.as_ref().ok_or_else(|| {
            Errors::DeficientTranslationContextError("Target meta context missing in translation context".to_string())
        })?;

        let contexts: Vec<Arc<Context>> = meta_context.contexts.values()
            .filter(|context| !context.network_name.is_empty())
            .cloned()
            .collect();

        let mut seen: HashSet<Lineage> = HashSet::new();
        let mut unique_contexts: Vec<Arc<Context>> = Vec::new();
        for context in contexts {
            if seen.insert(context.lineage.clone()) {
                unique_contexts.push(context);
            }
        }

        unique_contexts
};

    let input_contexts = {
        let lock = read_lock!(translation_context);
        let meta_context = lock.input_meta_context.as_ref().ok_or_else(|| {
            Errors::DeficientTranslationContextError("Input meta context missing in translation context".to_string())
        })?;

        let contexts: Vec<Arc<Context>> = meta_context.contexts.values()
            .filter(|context| !context.network_name.is_empty())
            .cloned()
            .collect();

        let mut seen: HashSet<Lineage> = HashSet::new();
        let mut unique_contexts: Vec<Arc<Context>> = Vec::new();
        for context in contexts {
            if seen.insert(context.lineage.clone()) {
                unique_contexts.push(context);
            }
        }

        unique_contexts
    };

    let context_pairs: Vec<(Arc<Context>, Arc<Context>)> = input_contexts.iter()
        .flat_map(|context_a| target_contexts.iter().map(move |context_b| {
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

            let maybe_translation_network = get_translation_network(
                cloned_provider,
                cloned_reasoner,
                cloned_translation_context,
                pair,
                &cloned_options,
                &cloned_stage_context,
            )
            .await?;
            
            Ok(maybe_translation_network)
        });
        handles.push(handle);
    }

    let results: Vec<Result<Option<TranslationNetwork>, Errors>> = try_join_all(handles).await?;

    let translation_networks: Vec<TranslationNetwork> = results.into_iter()
        .filter_map(|res| {
            match res {
                Ok(Some(translation_network)) => Some(Ok(translation_network)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<TranslationNetwork>, Errors>>()?;

    let hashmap: HashMap<ID, Arc<TranslationNetwork>> = translation_networks.into_iter()
        .map(|translation_network| {
            let translation_network = Arc::new(translation_network);
            let id = translation_network.id.clone();
            (id, translation_network)
        })
        .collect();

    Ok(hashmap)
}

async fn get_translation_network<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    translation_context: Arc<RwLock<TranslationContext>>,
    context_pair: (Arc<Context>, Arc<Context>),
    options: &Options,
    stage_context: &StageContext
) -> Result<Option<TranslationNetwork>, Errors> {
    let (input_context, target_context) = context_pair;

    if !options.regenerate {
        if let Some(maybe_translation_network) = provider.get_translation_network_by_lineages(
            &input_context.lineage,
            &target_context.lineage,
        ).await? {
            return Ok(maybe_translation_network);
        }
    }

    let (transformation, (tokens,)) = LLM::get_network_translation(
        Arc::clone(&translation_context),
        Arc::clone(&input_context),
        Arc::clone(&target_context),
    ).await?;

    if let Some(transformation) = transformation {
        let translation_network = TranslationNetwork {
            id: ID::new(),
            source_lineage: input_context.lineage.clone(),
            target_lineage: target_context.lineage.clone(),
            transformation: transformation.clone(),
        };

        provider.save_translation_network(
            (input_context.lineage.clone(), target_context.lineage.clone()),
            Some(translation_network.clone())
        ).await?;

        Ok(Some(translation_network))
    } else {
        provider.save_translation_network(
            (input_context.lineage.clone(), target_context.lineage.clone()),
            None
        ).await?;

        Ok(None)
    }
}

pub async fn get_classification<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<Arc<Classification>, Errors> {
    log::trace!("In get_classification");

    stage_context.record_events("Document classification", 0);

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };
    let lineage = read_lock!(meta_context.graph_root).lineage.clone();

    if !options.regenerate {
        if let Some(classification) = provider.get_classification_by_lineage(&lineage).await? {
            log::info!("Provider has supplied classification");

            return Ok(Arc::new(classification));
        };
    }

    let (classification, metadata) = reasoner.classify(
        Arc::clone(&meta_context)
    ).await?;

    provider
        .save_classification(&lineage, classification.clone())
        .await?;

    stage_context.record_events("Document classification", metadata.tokens.into());

    Ok(Arc::new(classification))
}
