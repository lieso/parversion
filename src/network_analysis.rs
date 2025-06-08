use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::task;
use tokio::sync::Semaphore;
use futures::future::try_join_all;

use crate::prelude::*;
use crate::provider::Provider;
use crate::graph_node::{Graph};
use crate::basis_network::{
    BasisNetwork,
    NetworkRelationship,
};
use crate::config::{CONFIG};
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::basis_graph::BasisGraph;

pub async fn get_basis_graph<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<Arc<BasisGraph>, Errors> {
    log::trace!("In get_basis_graph");

    let original_document = {
        let lock = read_lock!(meta_context);
        lock.get_original_document()
    };

    log::debug!("original_document: {}", original_document);

    delay();

    unimplemented!();

    let basis_graph = BasisGraph {
        id: ID::new(),
        name: "digest".to_string(),
        description: "A collection or summary of information, often curated or aggregated from various sources. It may be algorithmically curated or user generated.".to_string(),
    };
    
    Ok(Arc::new(basis_graph))
}

pub async fn get_basis_networks<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<ID, Arc<BasisNetwork>>, Errors> {
    log::trace!("In get_basis_networks");

    let graph_root = {
        let lock = read_lock!(meta_context);
        lock.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?
    };

    let mut queue = VecDeque::new();
    let mut unique_subgraphs = HashMap::new();

    queue.push_back(graph_root);

    while let Some(current) = queue.pop_front() {
        let current_read = read_lock!(current);

        if current_read.children.is_empty() {
            continue;
        }

        if !unique_subgraphs.contains_key(&current_read.subgraph_hash) {
            unique_subgraphs.insert(current_read.subgraph_hash.clone(), current.clone());
        }

        for child in &current_read.children {
            queue.push_back(child.clone());
        }
    }

    log::info!("Number of unique subgraphs: {:?}", unique_subgraphs.len());

    let max_concurrency = {
        let config_lock = read_lock!(CONFIG);
        config_lock.llm.max_concurrency
    };

    if max_concurrency == 1 {
        let mut results = HashMap::new();

        for subgraph in unique_subgraphs.values().cloned() {
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let result = get_basis_network(
                cloned_provider,
                cloned_meta_context,
                subgraph.clone()
            ).await?;

            results.insert(result.id.clone(), Arc::new(result));
        }

        Ok(results)
    } else {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for subgraph in unique_subgraphs.values().cloned() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);

            let handle = task::spawn(async move {
                let _permit = permit;
                let basis_network = get_basis_network(
                    cloned_provider,
                    cloned_meta_context,
                    subgraph.clone()
                ).await?;

                Ok((basis_network.id.clone(), Arc::new(basis_network)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(ID, Arc<BasisNetwork>), Errors>> = try_join_all(handles).await?;
        let hashmap_results: HashMap<ID, Arc<BasisNetwork>> = results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

async fn get_basis_network<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    graph: Graph
) -> Result<BasisNetwork, Errors> {
    log::trace!("In get_basis_network");

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts.clone().ok_or(Errors::ContextsNotProvided)?
    };
    let basis_graph = {
        let lock = read_lock!(meta_context);
        lock.basis_graph.clone().ok_or(Errors::BasisGraphNotProvided)?
    };

    let target_subgraph_hash = read_lock!(graph).subgraph_hash.clone();
    log::debug!("target_subgraph_hash: {}", target_subgraph_hash.to_string().unwrap());

    if read_lock!(graph).parents.is_empty() {
        log::info!("Node is root node; going to create null network for this node");

        return Ok(add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
    }

    if let Some(basis_network) = provider.get_basis_network_by_subgraph_hash(&target_subgraph_hash.to_string().unwrap()).await? {
        log::info!("Provider has supplied basis network");

        return Ok(basis_network);
    };

    log::info!("Generating json for siblings...");

    let parent: Graph = read_lock!(graph).parents
        .first()
        .unwrap()
        .clone();

    let sibling_contexts: Vec<_> = read_lock!(parent).children
        .iter()
        .map(|sibling| {
            contexts
                .get(&read_lock!(sibling).id)
                .unwrap()
                .clone()
        })
        .collect();

    log::info!("Number of sibling contexts: {}", sibling_contexts.len());

    let mut sibling_jsons = Vec::new();

    for (index, sibling_context) in sibling_contexts.iter().enumerate() {
        log::debug!("Processing sibling context {}/{}", index + 1, sibling_contexts.len());

        let mut sibling_json = sibling_context.generate_json(
            Arc::clone(&provider),
            Arc::clone(&meta_context)
        ).await?;

        

        let truncate_at = sibling_json.char_indices().nth(2000).map_or(sibling_json.len(), |(idx, _)| idx);
        sibling_json.truncate(truncate_at);



        let subgraph_hash = read_lock!(sibling_context.graph_node).subgraph_hash.clone();
        log::debug!("Other subgraph hash: {}", subgraph_hash);

        if sibling_json.is_empty() && subgraph_hash == target_subgraph_hash {
            log::info!("Target subgraph does not result in any meaningful JSON; we will not investigate it further.");

            return Ok(add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
        }

        if !sibling_json.is_empty() {
            sibling_jsons.push((subgraph_hash.to_string().unwrap().clone(), sibling_json));
        }
    }

    log::info!("Completed processing all sibling contexts.");

    if sibling_jsons.len() <= 1 {
        log::info!("Only one subgraph contains meaningful JSON; we will not investigate it further.");

        return Ok(add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
    }

    log::info!("Going to consult LLM for relationships between subgraphs...");

    let overall_context = basis_graph.description.clone();

    let (name, matches, description) = LLM::get_relationships(
        overall_context.clone(),
        target_subgraph_hash.to_string().unwrap().clone(),
        sibling_jsons.clone()
    ).await?;

    log::info!("Done asking LLM for relationships");

    if matches.is_empty() {
        log::info!("LLM did not find any relationships between subgraphs");

        return Ok(add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
    }

    log::info!("LLM determined subgraphs are associated: {:?}", matches);

    let associated_subgraphs = matches.iter().cloned()
        //.chain(std::iter::once(target_subgraph_hash.to_string().unwrap().clone()))
        .collect();

    let basis_network = BasisNetwork {
        id: ID::new(),
        description: description.clone(),
        subgraph_hash: target_subgraph_hash.to_string().unwrap().clone(),
        name: name.clone(),
        relationship: NetworkRelationship::Association(associated_subgraphs),
    };

    provider.save_basis_network(
        target_subgraph_hash.to_string().unwrap().clone(),
        basis_network.clone(),
    ).await?;

    Ok(basis_network)
}

async fn add_null_network<P: Provider>(
    provider: Arc<P>,
    target_subgraph_hash: Hash,
) -> Result<BasisNetwork, Errors> {
    let basis_network = BasisNetwork::new_null_network(
        &target_subgraph_hash.to_string().unwrap()
    );

    provider.save_basis_network(
        target_subgraph_hash.to_string().unwrap().clone(),
        basis_network.clone(),
    ).await?;

    Ok(basis_network)
}
