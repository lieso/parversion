use futures::future::try_join_all;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::classification::Classification;
use crate::basis_network::{BasisNetwork, BasisNetworkMetadata, NodeRelationship, NodeRelationshipType};
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

    let basis_nodes: Vec<Arc<BasisNode>> = {
        let lock = read_lock!(normalization_context);
        lock.basis_nodes
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(
                    "Basis nodes not provided in normalization context".to_string()
                )
            })?
            .values()
            .cloned()
            .collect()
    };

    log::info!("Number of basis nodes: {}", basis_nodes.len());

    let mut non_empty_basis_nodes: Vec<Arc<BasisNode>> = basis_nodes
        .iter()
        .filter(|basis_node| !basis_node.transformations.is_empty())
        .cloned()
        .collect();

    non_empty_basis_nodes.sort_by(|a, b| a.lineage.to_string().cmp(&b.lineage.to_string()));

    log::info!("Number of non-empty basis nodes: {}", non_empty_basis_nodes.len());

    let mut node_relationships: Vec<Arc<NodeRelationship>> = Vec::new();

    for i in 0..non_empty_basis_nodes.len() {
        let mut handles = Vec::new();

        for j in (i + 1)..non_empty_basis_nodes.len() {
            let left = Arc::clone(&non_empty_basis_nodes[i]);
            let right = Arc::clone(&non_empty_basis_nodes[j]);


            let has_transitivity = get_relationship(
                node_relationships.clone(),
                &left.lineage,
                &right.lineage,
            ).is_some();

            if has_transitivity {
                continue
            }

            let cloned_provider = Arc::clone(&provider);
            let cloned_reasoner = Arc::clone(&reasoner);
            let cloned_normalization_context = Arc::clone(&normalization_context);
            let cloned_stage_context = stage_context.clone();
            let cloned_options = options.clone();

            let handle = task::spawn(async move {
                generate_node_relationship(
                    cloned_provider,
                    cloned_reasoner,
                    cloned_normalization_context,
                    &cloned_options,
                    &cloned_stage_context,
                    left,
                    right,
                )
                .await
            });

            handles.push(handle);
        }

        let results = try_join_all(handles).await?;
        
        for result in results {
            node_relationships.push(Arc::new(result?));
        }
    }



    todo!("generate_basis_networks is being rewritten from scratch")
}

async fn generate_node_relationship<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<NodeRelationship, Errors> {

    if !options.regenerate {
        if let Some(node_relationship) = provider.get_node_relationship(
            &left.lineage,
            &right.lineage,
        ).await? {
            return Ok(node_relationship);
        }
    }

    stage_context.record_events("Node relationship", 0);

    let (relationship, metadata) = reasoner.node_relationship(
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone(),
    ).await?;

    stage_context.record_events("Node relationship", metadata.tokens.into());

    provider
        .save_node_relationship(
            left.lineage.clone(),
            right.lineage.clone(),
            relationship.clone()
        )
        .await?;

    Ok(relationship)
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

fn get_relationship(
    relationships: Vec<Arc<NodeRelationship>>,
    left_basis_lineage: &Lineage,
    right_basis_lineage: &Lineage,
) -> Option<NodeRelationshipType> {

    let left_relationships: Vec<Arc<NodeRelationship>> = relationships
        .iter()
        .filter(|relationship| {
            relationship.left_basis_lineage == *left_basis_lineage ||
            relationship.right_basis_lineage == *left_basis_lineage 
        })
        .cloned()
        .collect();

    if left_relationships.is_empty() {
        return None;
    }

    for relationship in left_relationships.iter() {
        if relationship.left_basis_lineage == *right_basis_lineage ||
            relationship.right_basis_lineage == *right_basis_lineage {
            return Some(relationship.relationship_type.clone());
        }
    }

    for relationship in left_relationships.iter() {
        if relationship.left_basis_lineage == *left_basis_lineage {
            if let Some(relationship_type) = get_relationship(
                relationships.clone(),
                &relationship.right_basis_lineage,
                right_basis_lineage,
            ) {
                return Some(relationship_type);
            }
        } else {
            if let Some(relationship_type) = get_relationship(
                relationships.clone(),
                &relationship.left_basis_lineage,
                right_basis_lineage,
            ) {
                return Some(relationship_type);
            }
        }
    }

    None
}
