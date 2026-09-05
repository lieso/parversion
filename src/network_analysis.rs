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

    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis node contexts not provided in normalization context".to_string())
            })?
    };

    let mut non_empty_basis_nodes: Vec<Arc<BasisNode>> = basis_nodes
        .iter()
        .filter(|basis_node| !basis_node.transformations.is_empty())
        .cloned()
        .collect();

    non_empty_basis_nodes.sort_by(|a, b| {
        let count_a = basis_node_contexts
            .get(&a.id)
            .unwrap()
            .len();
        let count_b = basis_node_contexts
            .get(&b.id)
            .unwrap()
            .len();

        count_b.cmp(&count_a)
    });

    log::info!("Number of non-empty basis nodes: {}", non_empty_basis_nodes.len());

    let mut node_relationships: Vec<Arc<NodeRelationship>> = Vec::new();

    for i in 0..non_empty_basis_nodes.len() {
        let mut handles = Vec::new();

        for j in (i+1)..non_empty_basis_nodes.len() {
            let left = Arc::clone(&non_empty_basis_nodes[i]);
            let right = Arc::clone(&non_empty_basis_nodes[j]);

            let is_reachable = has_reachability(
                &node_relationships,
                &left.lineage,
                &right.lineage,
            );

            if is_reachable {
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
            for relationship in result? {
                node_relationships.push(Arc::new(relationship));
            }
        }
    }

    let basis_networks = resolve_basis_networks(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        Arc::clone(&normalization_context),
        options,
        stage_context,
        non_empty_basis_nodes.clone(),
        node_relationships
    ).await?;

    let hashmap: HashMap<BasisNetworkID, Arc<BasisNetwork>> = basis_networks
        .into_iter()
        .map(|network| (network.id.clone(), network))
        .collect();

    Ok((hashmap,))
}

async fn resolve_basis_networks<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    basis_nodes: Vec<Arc<BasisNode>>,
    relationships: Vec<Arc<NodeRelationship>>,
) -> Result<Vec<Arc<BasisNetwork>>, Errors> {
    let actual_relationships: Vec<Arc<NodeRelationship>> = relationships
        .iter()
        .filter(|rel| {
            matches!(rel.relationship_type, NodeRelationshipType::Equal { .. } | NodeRelationshipType::Combine { .. })
        })
        .cloned()
        .collect();

    let mut basis_networks: Vec<Arc<BasisNetwork>> = Vec::new();
    let mut placed: HashSet<Lineage> = HashSet::new();
    let mut handles = Vec::new();

    for basis_node in &basis_nodes {
        if placed.contains(&basis_node.lineage) {
            continue;
        }

        let current_relationships = get_node_relationships(
            actual_relationships.clone(),
            &basis_node.lineage
        );

        let mut basis_network_nodes: Vec<Arc<BasisNode>> = Vec::new();

        for relationship in &current_relationships {
            let lineages = vec![relationship.left_basis_lineage.clone(), relationship.right_basis_lineage.clone()];

            for lineage in lineages {
                if placed.contains(&lineage) {
                    continue;
                }

                let node: Arc<BasisNode> = basis_nodes
                    .iter()
                    .find(|item| item.lineage == lineage)
                    .unwrap()
                    .clone();

                basis_network_nodes.push(node);
                placed.insert(lineage.clone());
            }
        }

        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();

        let handle = task::spawn(async move {
            generate_basis_network(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                &cloned_options,
                &cloned_stage_context,
                basis_network_nodes,
                current_relationships.clone(),
            ).await
        });
        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    let basis_networks: Vec<Arc<BasisNetwork>> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|basis_network| {
            Arc::new(basis_network)
        })
        .collect();

    Ok(basis_networks)
}

async fn generate_basis_network<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    basis_nodes: Vec<Arc<BasisNode>>,
    relationships: Vec<Arc<NodeRelationship>>
) -> Result<BasisNetwork, Errors> {
    stage_context.record_events("Network analysis", 0);

    if !options.regenerate {
        if let Some(basis_network) = provider.get_basis_network(
            basis_nodes.clone()
        ).await? {
            return Ok(basis_network);
        }
    }

    let (basis_network, metadata) = reasoner.basis_network(
        Arc::clone(&normalization_context),
        basis_nodes.clone()
    ).await?;

    stage_context.record_events("Network analysis", metadata.tokens.into());

    provider
        .save_basis_network(
            basis_nodes.clone(),
            basis_network.clone()
        )
        .await?;

    Ok(basis_network)
}

async fn generate_node_relationship<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<Vec<NodeRelationship>, Errors> {

    stage_context.record_events("Node relationship", 0);

    if !options.regenerate {
        if let Some(node_relationships) = provider.get_node_relationships(
            &left.lineage,
            &right.lineage,
        ).await? {
            return Ok(node_relationships);
        }
    }

    let results = reasoner.node_relationship(
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone(),
    ).await?;

    let total_tokens: u32 = results.iter().map(|(_, metadata)| metadata.tokens).sum();
    stage_context.record_events("Node relationship", total_tokens.into());

    let relationships: Vec<NodeRelationship> = results.into_iter().map(|(r, _)| r).collect();

    provider
        .save_node_relationships(
            left.lineage.clone(),
            right.lineage.clone(),
            relationships.clone()
        )
        .await?;

    Ok(relationships)
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

fn get_node_relationships(
    relationships: Vec<Arc<NodeRelationship>>,
    basis_lineage: &Lineage
) -> Vec<Arc<NodeRelationship>> {
    let mut visited_lineages: HashSet<Lineage> = HashSet::new();
    let mut queue: VecDeque<Lineage> = VecDeque::new();
    let mut collected: HashMap<ID, Arc<NodeRelationship>> = HashMap::new();

    visited_lineages.insert(basis_lineage.clone());
    queue.push_back(basis_lineage.clone());

    while let Some(current) = queue.pop_front() {
        for relationship in &relationships {
            if relationship.left_basis_lineage == current || relationship.right_basis_lineage == current {
                collected.entry(relationship.id.clone()).or_insert_with(|| Arc::clone(relationship));

                let neighbour = if relationship.left_basis_lineage == current {
                    relationship.right_basis_lineage.clone()
                } else {
                    relationship.left_basis_lineage.clone()
                };

                if visited_lineages.insert(neighbour.clone()) {
                    queue.push_back(neighbour);
                }
            }
        }
    }

    collected.into_values().collect()
}

fn has_reachability(
    relationships: &Vec<Arc<NodeRelationship>>,
    left_basis_lineage: &Lineage,
    right_basis_lineage: &Lineage,
) -> bool {
    fn recurse(
        relationships: &Vec<Arc<NodeRelationship>>,
        current: &Lineage,
        target: &Lineage,
        visited: &mut HashSet<Lineage>,
    ) -> bool {
        if current == target {
            return true;
        }
        
        visited.insert(current.clone());

        for relationship in relationships {
            let neighbour = {
                match relationship.relationship_type {
                    NodeRelationshipType::Combine {  .. } => {
                        if relationship.left_basis_lineage == *current {
                            &relationship.right_basis_lineage
                        } else if relationship.right_basis_lineage == *current {
                            &relationship.left_basis_lineage
                        } else {
                            continue;
                        }
                    },
                    NodeRelationshipType::Equal { .. } => {




                        // hmm....
                        if relationship.left_basis_lineage == *current {
                            &relationship.right_basis_lineage
                        } else if relationship.right_basis_lineage == *current {
                            &relationship.left_basis_lineage
                        } else {
                            continue;
                        }





                    },
                    NodeRelationshipType::NoRelationship => {
                        continue;
                    }
                }
            };

            if !visited.contains(neighbour) {
                if recurse(
                    relationships,
                    neighbour,
                    target,
                    visited,
                ) {
                    return true;
                }
            }
        }

        false
    }

    recurse(
        relationships,
        left_basis_lineage,
        right_basis_lineage,
        &mut HashSet::new()
    )
}
