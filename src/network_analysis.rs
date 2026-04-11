use futures::future::try_join_all;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;
use serde_json::{json, Value, Map};

use crate::classification::Classification;
use crate::basis_network::{BasisNetwork, NetworkType};
use crate::basis_graph::BasisGraph;
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::transformation::{
    NetworkTransformation,
    CanonicalizationTransformation,
    RelationshipTransformation,
    ResolvedRelationshipTransformation,
    TraversalTransformation
};
use crate::network_relationship::{NetworkRelationship, NetworkRelationshipType};
use crate::json_node::JsonNode;

pub async fn get_classification<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<Arc<Classification>, Errors> {
    log::trace!("In get_classification");

    stage_context.record_events("Document classification", 0);

    let original_document = {
        let lock = read_lock!(meta_context);
        lock.get_original_document()
    };
    let graph_root = {
        let lock = read_lock!(meta_context);
        lock.graph_root
            .clone()
            .ok_or(Errors::GraphRootNotProvided)?
    };
    let lineage = read_lock!(graph_root).lineage.clone();

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
    };

    provider
        .save_classification(&lineage, classification.clone())
        .await?;

    stage_context.record_events("Document classification", tokens);

    Ok(Arc::new(classification))
}

pub async fn get_network_relationships<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<NetworkRelationship>>, Errors> {
    log::trace!("In get_network_relationships");

    let unique_subgraphs: HashMap<Hash, Vec<Graph>> = get_unique_subgraphs(Arc::clone(&meta_context));

    let complex_networks: Vec<Arc<BasisNetwork>> = unique_subgraphs
        .into_iter()
        .filter_map(|(subgraph_hash, _)| {
            let meta_context_lock = read_lock!(meta_context);
            match meta_context_lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash) {
                Ok(Some(basis_network)) => {
                    match &basis_network.transformation {
                        crate::basis_network::NetworkType::Complex(_) => Some(basis_network),
                        _ => None,
                    }
                }
                _ => None,
            }
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


    log::debug!("graph hash: {}", graph_hash);








    let basis_graph: BasisGraph = get_canonical_networks(
        Arc::clone(&provider),
        Arc::clone(&meta_context),
        options,
        stage_context,
        &graph_hash,
        complex_networks.clone(),
    ).await?;
    let canonical_networks: Vec<Arc<BasisNetwork>> = basis_graph.canonicalization.transform(complex_networks)?;

    if canonical_networks.is_empty() {
        panic!("Canonical networks not found?");
    }









    let basis_graph: BasisGraph = get_relationship_typing(
        Arc::clone(&provider),
        Arc::clone(&meta_context),
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
        panic!("No relationships?");
    }

    let max_concurrency = {
        let config_lock = read_lock!(CONFIG);
        config_lock.llm.max_concurrency
    };

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for resolved_relationship in relationships {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();
        let cloned_graph_hash = graph_hash.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            get_traversal(
                cloned_provider,
                cloned_meta_context,
                resolved_relationship,
                &cloned_options,
                &cloned_stage_context,
                &cloned_graph_hash,
            ).await
        });
        handles.push(handle);
    }

    let results: Vec<Result<(), Errors>> = try_join_all(handles).await?;

    unimplemented!()
}

async fn get_traversal<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    resolved_relationship: ResolvedRelationshipTransformation,
    options: &Options,
    stage_context: &StageContext,
    graph_hash: &Hash,
) -> Result<(), Errors> {
    log::trace!("In get_traversal");

    match resolved_relationship.relationship_type {
        NetworkRelationshipType::Composition => {

            stage_context.record_events("Composition linking", 0);

            if let Some(mut basis_graph) = provider.get_basis_graph_by_hash(graph_hash).await? {
                if !options.regenerate {
                    if let Some(traversals) = &basis_graph.traversals {
                        if traversals.iter().any(|t| t.relationship_id == resolved_relationship.id) {
                            log::info!("TraversalTransformation already exists for this relationship");
                            return Ok(());
                        }
                    }
                }

                let (xpath, name, (tokens,)) = NetworkRelationship::process_composition(
                    Arc::clone(&meta_context),
                    Arc::clone(&resolved_relationship.from),
                    Arc::clone(&resolved_relationship.to),
                ).await?;

                let traversal = TraversalTransformation {
                    id: ID::new(),
                    relationship_id: resolved_relationship.id.clone(),
                    xpath,
                    name,
                    description: String::new(),
                };

                if basis_graph.traversals.is_none() {
                    basis_graph.traversals = Some(Vec::new());
                }
                if let Some(ref mut traversals) = basis_graph.traversals {
                    traversals.push(traversal);
                }

                stage_context.record_events("Composition linking", tokens);

                provider.save_basis_graph(graph_hash, basis_graph).await?;
            }

        }
        NetworkRelationshipType::ParentChild => {
            NetworkRelationship::process_parent_child(
                Arc::clone(&meta_context),
                Arc::clone(&resolved_relationship.from),
                Arc::clone(&resolved_relationship.to),
            ).await?;
        }
        _ => {
            log::warn!("Ignoring relationship type: {:?}", resolved_relationship.relationship_type);
        }
    }

    Ok(())
}

pub async fn get_relationship_typing<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
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
            Arc::clone(&meta_context),
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
    meta_context: Arc<RwLock<MetaContext>>,
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
        Arc::clone(&meta_context),
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
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisNetwork>>, Errors> {
    log::trace!("In get_basis_networks");

    let unique_subgraphs: HashMap<Hash, Vec<Graph>> = get_unique_subgraphs(Arc::clone(&meta_context));

    let all_subgraph_hashes = Arc::new(
        unique_subgraphs
            .keys()
            .map(|hash| hash.to_string().unwrap())
            .collect::<Vec<String>>()
    );

    let document_summary = {
        let lock = read_lock!(meta_context);
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
        let cloned_meta_context = Arc::clone(&meta_context);
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
    meta_context: Arc<RwLock<MetaContext>>,
    subgraph_hash: Hash,
    graphs: Vec<Graph>,
    options: &Options,
    stage_context: &StageContext,
    document_summary: &str,
    all_subgraph_hashes: Arc<Vec<String>>,
    lineage: Lineage,
) -> Result<BasisNetwork, Errors> {
    log::trace!("In get_basis_network");

    stage_context.record_events("Network analysis", 0);

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts.clone().ok_or(Errors::ContextsNotProvided)?
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
        let context = contexts.get(&read_lock!(graph).id).unwrap().clone();
        let json = context.generate_json_snippet(
            Arc::clone(&meta_context)
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

    let network: NetworkType = {
        if complex_json.is_empty() {
            NetworkType::Degenerate
        } else {
            let (network_transformation, (tokens)) = LLM::get_network_transformation(
                &subgraph_hash.to_string().unwrap(),
                &complex_json,
                document_summary
            ).await?;

            stage_context.record_events("Network analysis", tokens);

            NetworkType::Complex(network_transformation)
        }
    };

    let description = {
        match network {
            NetworkType::Degenerate => String::from("Degenerate network"),
            NetworkType::Complex(ref network_transformation) => network_transformation.description.clone(),
        }
    };

    let basis_network = BasisNetwork {
        id: ID::new(),
        description,
        subgraph_hash: subgraph_hash.clone(),
        lineage: lineage.clone(),
        transformation: network,
    };

    provider
        .save_basis_network(&lineage, &subgraph_hash, basis_network.clone())
        .await?;

    Ok(basis_network)
}

fn get_unique_subgraphs(meta_context: Arc<RwLock<MetaContext>>) -> HashMap<Hash, Vec<Graph>> {
    let graph_root = {
        let lock = read_lock!(meta_context);
        lock.graph_root.as_ref().unwrap().clone()
    };

    let mut queue = VecDeque::new();
    let mut unique_subgraphs: HashMap<Hash, Vec<Graph>> = HashMap::new();

    queue.push_back(graph_root);

    while let Some(current) = queue.pop_front() {
        let lock = read_lock!(current);

        let subgraph_hash = lock.subgraph_hash.clone();

        log::debug!("unique subgraph: {}", &subgraph_hash);

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
