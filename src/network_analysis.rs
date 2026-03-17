use futures::future::try_join_all;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::basis_graph::BasisGraph;
use crate::basis_network::{BasisNetwork};
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::provider::Provider;

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

pub async fn get_basis_networks<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisNetwork>>, Errors> {
    log::trace!("In get_basis_networks");

    let unique_subgraphs: HashMap<Hash, Graph> = get_unique_subgraphs(Arc::clone(&meta_context));

    let max_concurrency = {
        let config_lock = read_lock!(CONFIG);
        config_lock.llm.max_concurrency
    };

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for subgraph in unique_subgraphs.values().cloned() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_subgraph = Arc::clone(&subgraph);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let _permit = permit;
            let basis_network = get_basis_network(
                cloned_provider,
                cloned_meta_context,
                cloned_subgraph,
                &cloned_options,
                &cloned_stage_context,
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
    graph: Graph,
    options: &Options,
    stage_context: &StageContext
) -> Result<BasisNetwork, Errors> {
    log::trace!("In get_basis_network");

    stage_context.record_events("Network analysis", 0);

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts.clone().ok_or(Errors::ContextsNotProvided)?
    };

    let lineage = read_lock!(graph).lineage.clone();
    let subgraph_hash = read_lock!(graph).subgraph_hash.clone();
    let description = read_lock!(graph).description.clone();

    //if !options.regenerate {
    //    if let Some(basis_network) = provider.get_basis_network_by_lineage_and_subgraph_hash(
    //        &lineage,
    //        &subgraph_hash
    //    ).await? {
    //        return Ok(basis_network);
    //    }
    //}






    let context = contexts.get(&read_lock!(graph).id).unwrap().clone();



    










    





    //let (_, (tokens)) = LLM::get_network_transformations(

    //).await?;

    unimplemented!();

    let basis_network = BasisNetwork {
        id: ID::new(),
        description,
        subgraph_hash,
        lineage,
        // transformations:
    };

    //provider
    //    .save_basis_network(&lineage, &subgraph_hash, basis_network.clone())
    //    .await?;

    stage_context.record_events("Network analyis", 0);

    Ok(basis_network)
}

fn get_unique_subgraphs(meta_context: Arc<RwLock<MetaContext>>) -> HashMap<Hash, Graph> {
    let graph_root = {
        let lock = read_lock!(meta_context);
        lock.graph_root.as_ref().unwrap().clone()
    };

    let mut queue = VecDeque::new();
    let mut unique_subgraphs = HashMap::new();

    queue.push_back(graph_root);

    while let Some(current) = queue.pop_front() {
        let lock = read_lock!(current);

        if lock.children.is_empty() {
            continue;
        }

        if !unique_subgraphs.contains_key(&lock.subgraph_hash) {
            unique_subgraphs.insert(lock.subgraph_hash.clone(), current.clone());
        }

        for child in &lock.children {
            queue.push_back(child.clone());
        }
    }

    log::debug!("Number of unique subgraphs: {:?}", unique_subgraphs.len());

    unique_subgraphs
}
