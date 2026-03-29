use futures::future::try_join_all;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;
use serde_json::{json, Value, Map};

use crate::basis_graph::BasisGraph;
use crate::basis_network::{BasisNetwork, NetworkType};
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::provider::Provider;
use crate::transformation::NetworkTransformation;
use crate::network_relationship::NetworkRelationship;
use crate::json_node::JsonNode;

pub async fn get_basis_graph<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<Arc<BasisGraph>, Errors> {
    log::trace!("In get_basis_graph");

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
        if let Some(basis_graph) = provider.get_basis_graph_by_lineage(&lineage).await? {
            log::info!("Provider has supplied basis graph");

            return Ok(Arc::new(basis_graph));
        };
    }

    let (name, description, structure, aliases, tokens) = LLM::categorize(original_document).await?;

    let basis_graph = BasisGraph {
        id: ID::new(),
        name,
        aliases,
        description,
        structure,
        lineage: lineage.clone(),
    };

    provider
        .save_basis_graph(&lineage, basis_graph.clone())
        .await?;

    stage_context.record_events("Document classification", tokens);

    Ok(Arc::new(basis_graph))
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

    stage_context.record_events("Finding canonical networks", 0);

    let (canonical_networks, (tokens,)) = NetworkRelationship::get_canonical_networks(
        Arc::clone(&provider),
        Arc::clone(&meta_context),
        complex_networks
    ).await?;

    stage_context.record_events("Finding canonical networks", tokens);

    unimplemented!()
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
        let basis_graph = lock.get_basis_graph().unwrap();

        format!(r##"
            [name]
            {}

            [description]
            {}

            [structure]
            {}
        "##, basis_graph.name, basis_graph.description, basis_graph.structure)
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
