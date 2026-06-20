use futures::future::try_join_all;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::LLM;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::transformation::{
    CanonicalizationTransformation,
    RelationshipTransformation,
    ResolvedRelationshipTransformation,
    TraversalTransformation
};
use crate::network_relationship::{NetworkRelationship, NetworkRelationshipType};
use crate::traversal::{get_original_document_condensed};
use crate::translation_network::TranslationNetwork;

pub async fn get_translation_networks<P: Provider>(
    provider: Arc<P>,
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
        let cloned_translation_context = Arc::clone(&translation_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;

            let maybe_translation_network = get_translation_network(
                cloned_provider,
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

async fn get_translation_network<P: Provider>(
    provider: Arc<P>,
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

pub async fn get_classification<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<Arc<Classification>, Errors> {
    log::trace!("In get_classification");

    stage_context.record_events("Document classification", 0);

    let original_document = get_original_document_condensed(Arc::clone(&normalization_context))?;
    let graph_root = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .as_ref()
            .ok_or(Errors::DeficientNormalizationContextError("Graph root not provided in normalization context".to_string()))?
            .graph_root
            .clone()
    };
    let lineage = read_lock!(graph_root).lineage.clone();
    let acyclic_subgraph_hash: Hash = read_lock!(graph_root).acyclic_subgraph_hash();

    if !options.regenerate {
        if let Some(classification) = provider.get_classification_by_lineage(&lineage).await? {
            log::info!("Provider has supplied classification");

            return Ok(Arc::new(classification));
        };
    }

    let (name, description, structure, aliases, tokens) = LLM::categorize(original_document).await?;

    let classification = Classification {
        id: ID::new(),
        name,
        aliases,
        description,
        structure,
        lineage: lineage.clone(),
        acyclic_subgraph_hash: acyclic_subgraph_hash.clone(),
    };

    provider
        .save_classification(&lineage, classification.clone())
        .await?;

    stage_context.record_events("Document classification", tokens);

    Ok(Arc::new(classification))
}

pub async fn get_network_relationships<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<BasisGraph, Errors> {
    log::trace!("In get_network_relationships");

    let complex_networks: Vec<Arc<BasisNetwork>> = collect_complex_unique_subgraphs(Arc::clone(&normalization_context))
        .into_iter()
        .filter_map(|(subgraph_hash, _)| {
            let meta_context_lock = read_lock!(normalization_context);
            meta_context_lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)
                .ok()
                .flatten()
        })
        .collect();

    log::debug!("Complex networks found: {}", complex_networks.len());

    let mut graph_hash = Hash::from_items(
        complex_networks
        .iter()
        .map(|network| { network.subgraph_hash.to_string().unwrap() })
        .collect()
    );
    graph_hash.sort();
    graph_hash.finalize();

    log::info!("graph hash: {}", graph_hash);

    // ═════════════════════════════════════════════════════════════════════════════════
    // STAGE 1: CANONICALIZE NETWORKS
    // ═════════════════════════════════════════════════════════════════════════════════

    let basis_graph: BasisGraph = get_canonical_networks(
        Arc::clone(&provider),
        Arc::clone(&normalization_context),
        options,
        stage_context,
        &graph_hash,
        complex_networks.clone(),
    ).await?;
    let canonical_networks: Vec<Arc<BasisNetwork>> = basis_graph.canonicalization.transform(complex_networks)?;

    if canonical_networks.is_empty() {
        panic!("Canonical networks not found?");
    }

    // ═════════════════════════════════════════════════════════════════════════════════
    // STAGE 2: IDENTIFY RELATIONSHIP TYPES
    // ═════════════════════════════════════════════════════════════════════════════════

    let basis_graph: BasisGraph = get_relationship_typing(
        Arc::clone(&provider),
        Arc::clone(&normalization_context),
        options,
        stage_context,
        &graph_hash,
        canonical_networks.clone(),
    ).await?;

    let relationships: Vec<ResolvedRelationshipTransformation> = basis_graph
        .relationships
        .unwrap_or_default()
        .iter()
        .map(|rel_transform| rel_transform.transform(&canonical_networks))
        .collect::<Result<Vec<_>, _>>()?;

    if relationships.is_empty() {
        //panic!("No relationships?");
    }

    // ═════════════════════════════════════════════════════════════════════════════════
    // STAGE 3: PROCESS RELATIONSHIP TRAVERSALS
    // ═════════════════════════════════════════════════════════════════════════════════

    let max_concurrency = {
        let config_lock = read_lock!(CONFIG);
        config_lock.llm.max_concurrency
    };

    let existing_relationship_ids: HashSet<ID> = if !options.regenerate {
        provider.get_basis_graph_by_hash(&graph_hash).await?
            .and_then(|bg| bg.traversals)
            .map(|ts| ts.into_iter().map(|t| t.relationship_id).collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for resolved_relationship in relationships {
        if existing_relationship_ids.contains(&resolved_relationship.id) {
            log::info!("TraversalTransformation already exists for this relationship");
            continue;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_meta_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;

            get_traversal(
                cloned_meta_context,
                resolved_relationship,
                &cloned_stage_context,
            ).await
        });
        handles.push(handle);
    }

    let new_traversals: Vec<TraversalTransformation> = try_join_all(handles).await?
        .into_iter()
        .collect::<Result<Vec<_>, Errors>>()?
        .into_iter()
        .flatten()
        .collect();

    if !new_traversals.is_empty() {
        let mut basis_graph = provider.get_basis_graph_by_hash(&graph_hash).await?
            .ok_or(Errors::UnexpectedError)?;
        basis_graph.traversals.get_or_insert_with(Vec::new).extend(new_traversals);
        provider.save_basis_graph(&graph_hash, basis_graph).await?;
    }

    provider.get_basis_graph_by_hash(&graph_hash).await?
        .ok_or(Errors::UnexpectedError)
}

async fn get_traversal(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    resolved_relationship: ResolvedRelationshipTransformation,
    stage_context: &StageContext,
) -> Result<Option<TraversalTransformation>, Errors> {
    log::trace!("In get_traversal");

    match resolved_relationship.relationship_type {
        NetworkRelationshipType::Composition => {
            stage_context.record_events("Composition linking", 0);

            let (traversal, name, (tokens,)) = NetworkRelationship::process_composition(
                Arc::clone(&normalization_context),
                Arc::clone(&resolved_relationship.from),
                Arc::clone(&resolved_relationship.to),
            ).await?;

            stage_context.record_events("Composition linking", tokens);

            Ok(Some(TraversalTransformation {
                id: ID::new(),
                relationship_id: resolved_relationship.id.clone(),
                traversal,
                name,
                description: String::new(),
            }))
        }
        NetworkRelationshipType::ParentChild => {
            stage_context.record_events("Parent-child linking", 0);

            let (traversal, (tokens,)) = NetworkRelationship::process_parent_child(
                Arc::clone(&normalization_context),
                Arc::clone(&resolved_relationship.from),
                Arc::clone(&resolved_relationship.to),
            ).await?;

            stage_context.record_events("Parent-child linking", tokens);

            Ok(Some(TraversalTransformation {
                id: ID::new(),
                relationship_id: resolved_relationship.id.clone(),
                traversal,
                name: String::new(),
                description: String::new(),
            }))
        }
        _ => {
            log::warn!("Ignoring relationship type: {:?}", resolved_relationship.relationship_type);
            Ok(None)
        }
    }
}

pub async fn get_relationship_typing<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    graph_hash: &Hash,
    canonical_networks: Vec<Arc<BasisNetwork>>
) -> Result<BasisGraph, Errors> {
    log::trace!("In get_relationship_typing");

    stage_context.record_events("Relationship typing", 0);

    if let Some(basis_graph) = provider.get_basis_graph_by_hash(graph_hash).await? {
        if !options.regenerate {
            if basis_graph.relationships.is_some() {
                return Ok(basis_graph);
            }
        }

        let (typed_relationships, (tokens,)) = NetworkRelationship::get_relationship_typing(
            Arc::clone(&normalization_context),
            canonical_networks.clone()
        ).await?;

        let mut basis_graph = basis_graph;
        basis_graph.relationships = Some(
            typed_relationships.into_iter()
                .map(|(from, to, rel_type)| {
                    RelationshipTransformation {
                        id: ID::new(),
                        from: from.id.clone(),
                        to: to.id.clone(),
                        relationship_type: rel_type,
                        description: String::new(),
                    }
                })
                .collect()
        );

        stage_context.record_events("Relationship typing", tokens);

        provider.save_basis_graph(graph_hash, basis_graph.clone()).await?;

        Ok(basis_graph)
    } else {
        log::error!("Trying to obtain relationship typing among canonical networks, but canonical networks were not found.");
        Err(Errors::UnexpectedError)
    }
}

pub async fn get_canonical_networks<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    graph_hash: &Hash,
    complex_networks: Vec<Arc<BasisNetwork>>
) -> Result<BasisGraph, Errors> {
    log::trace!("In get_canonical_networks");

    stage_context.record_events("Finding canonical networks", 0);

    if !options.regenerate {
        if let Some(basis_graph) = provider.get_basis_graph_by_hash(graph_hash).await? {
            return Ok(basis_graph);
        }
    }

    let (canonical_networks, (tokens,)) = NetworkRelationship::get_canonical_networks(
        Arc::clone(&normalization_context),
        complex_networks
    ).await?;
    let canonical_networks: Vec<Arc<BasisNetwork>> = canonical_networks.into_iter().map(Arc::new).collect();

    stage_context.record_events("Finding canonical networks", tokens);

    let basis_graph = BasisGraph {
        id: ID::new(),
        hash: graph_hash.clone(),
        canonicalization: CanonicalizationTransformation {
            id: ID::new(),
            canonical_networks: canonical_networks
                .iter()
                .map(|network| {
                    network.subgraph_hash.to_string().unwrap()
                })
                .collect()
        },
        relationships: None,
        traversals: None,
    };

    provider.save_basis_graph(graph_hash, basis_graph.clone()).await?;
    
    Ok(basis_graph)
}

pub async fn get_basis_networks<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisNetwork>>, Errors> {
    log::trace!("In get_basis_networks");

    let unique_subgraphs: HashMap<Hash, Vec<Graph>> = collect_complex_unique_subgraphs(Arc::clone(&normalization_context));

    let all_subgraph_hashes = Arc::new(
        unique_subgraphs
            .keys()
            .map(|hash| hash.to_string().unwrap())
            .collect::<Vec<String>>()
    );

    let document_summary = {
        let lock = read_lock!(normalization_context);
        let classification = lock.get_classification().unwrap();

        format!(r##"
            [name]
            {}

            [description]
            {}

            [structure]
            {}
        "##, classification.name, classification.description, classification.structure)
    };
    let document_summary_string = Arc::new(document_summary);

    let max_concurrency = {
        let config_lock = read_lock!(CONFIG);
        config_lock.llm.max_concurrency
    };

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for (subgraph_hash, graphs) in unique_subgraphs {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&normalization_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();
        let cloned_document_summary = Arc::clone(&document_summary_string);
        let cloned_subgraph_hashes = Arc::clone(&all_subgraph_hashes);

        let lineage = read_lock!(&graphs[0]).lineage.clone();

        let handle = task::spawn(async move {
            let _permit = permit;

            let basis_network = get_basis_network(
                cloned_provider,
                cloned_meta_context,
                subgraph_hash,
                graphs,
                &cloned_options,
                &cloned_stage_context,
                &cloned_document_summary,
                cloned_subgraph_hashes,
                lineage,
            )
            .await?;

            Ok((basis_network.id.clone(), Arc::new(basis_network)))
        });
        handles.push(handle);
    }

    let results: Vec<Result<(ID, Arc<BasisNetwork>), Errors>> =
        try_join_all(handles).await?;
    let hashmap_results: HashMap<ID, Arc<BasisNetwork>> =
        results.into_iter().collect::<Result<_, _>>()?;

    Ok(hashmap_results)
}

async fn get_basis_network<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    subgraph_hash: Hash,
    graphs: Vec<Graph>,
    options: &Options,
    stage_context: &StageContext,
    document_summary: &str,
    _all_subgraph_hashes: Arc<Vec<String>>,
    lineage: Lineage,
) -> Result<BasisNetwork, Errors> {
    stage_context.record_events("Network analysis", 0);

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    if !options.regenerate {
        if let Some(basis_network) = provider.get_basis_network_by_lineage_and_subgraph_hash(
            &lineage,
            &subgraph_hash
        ).await? {
            return Ok(basis_network);
        }
    }

    let mut complex_json: Vec<String> = Vec::new();

    for graph in graphs.iter().take(5) {
        let context = meta_context.contexts_lookup.get(&read_lock!(graph).id).unwrap().clone();
        let json = context.generate_json_snippet(
            Arc::clone(&normalization_context)
        )?;

        let subgraph_hash_string = subgraph_hash.to_string().unwrap();
        log::debug!("subgraph_hash_string: {}", subgraph_hash_string);

        if json.len() > 1 {
            let json_string = serde_json::to_string_pretty(&json).expect("Could not make a JSON string");
            log::debug!("{}", json_string);

            complex_json.push(json_string);
        } else {
            log::info!("DEGENERATE NETWORK");
        }
    }

    if complex_json.is_empty() {
        log::warn!("Degenerate network reached get_basis_network — should have been filtered by collect_complex_unique_subgraphs");
        return Err(Errors::UnexpectedError);
    }

    let (network_transformation, tokens) = LLM::get_network_transformation(
        &subgraph_hash.to_string().unwrap(),
        &complex_json,
        document_summary
    ).await?;

    stage_context.record_events("Network analysis", tokens);

    let basis_network = BasisNetwork {
        id: ID::new(),
        description: network_transformation.description.clone(),
        subgraph_hash: subgraph_hash.clone(),
        lineage: lineage.clone(),
        transformation: network_transformation,
    };

    provider
        .save_basis_network(&lineage, &subgraph_hash, basis_network.clone())
        .await?;

    Ok(basis_network)
}

fn collect_complex_unique_subgraphs(normalization_context: Arc<RwLock<NormalizationContext>>) -> HashMap<Hash, Vec<Graph>> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone()
    };

    get_unique_subgraphs(Arc::clone(&normalization_context))
        .into_iter()
        .filter(|(_, graphs)| {
            graphs.iter().take(5).any(|graph| {
                let graph_id = read_lock!(graph).id.clone();
                if let Some(context) = meta_context.as_ref().and_then(|m| m.contexts_lookup.get(&graph_id)) {
                    if let Ok(json) = context.generate_json_snippet(Arc::clone(&normalization_context)) {
                        return json.len() > 1;
                    }
                }
                false
            })
        })
        .collect()
}

fn get_unique_subgraphs(normalization_context: Arc<RwLock<NormalizationContext>>) -> HashMap<Hash, Vec<Graph>> {
    let graph_root = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.as_ref().unwrap().graph_root.clone()
    };

    let mut queue = VecDeque::new();
    let mut unique_subgraphs: HashMap<Hash, Vec<Graph>> = HashMap::new();

    queue.push_back(graph_root);

    while let Some(current) = queue.pop_front() {
        let lock = read_lock!(current);

        let subgraph_hash = lock.subgraph_hash.clone();

        unique_subgraphs
            .entry(subgraph_hash)
            .or_insert_with(Vec::new)
            .push(current.clone());

        for child in &lock.children {
            queue.push_back(child.clone());
        }
    }

    log::debug!("Number of unique subgraphs: {:?}", unique_subgraphs.len());

    unique_subgraphs
}
