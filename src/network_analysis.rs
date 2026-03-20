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
use crate::transformation::NetworkTransformation;
use crate::network_relationship::NetworkRelationship;

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

    let unique_subgraphs: HashMap<Hash, Graph> = get_unique_subgraphs(Arc::clone(&meta_context));

    let networks_with_transformations: Vec<Arc<BasisNetwork>> = unique_subgraphs
        .into_iter()
        .filter_map(|(_, graph)| {
            let subgraph_hash = read_lock!(graph).subgraph_hash.clone();
            let meta_context_lock = read_lock!(meta_context);
            match meta_context_lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash) {
                Ok(Some(basis_network)) if basis_network.network_transformation.is_some() => Some(basis_network),
                _ => None,
            }
        })
        .collect();

    log::debug!("Networks with transformations: {}", networks_with_transformations.len());

    unimplemented!()
}

pub async fn get_basis_networks<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisNetwork>>, Errors> {
    log::trace!("In get_basis_networks");

    let unique_subgraphs: HashMap<Hash, Graph> = get_unique_subgraphs(Arc::clone(&meta_context));

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

    for subgraph in unique_subgraphs.values().cloned() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_subgraph = Arc::clone(&subgraph);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();
        let cloned_document_summary = Arc::clone(&document_summary_string);

        let handle = task::spawn(async move {
            let _permit = permit;
            let basis_network = get_basis_network(
                cloned_provider,
                cloned_meta_context,
                cloned_subgraph,
                &cloned_options,
                &cloned_stage_context,
                &cloned_document_summary,
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
    stage_context: &StageContext,
    document_summary: &str
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

    if !options.regenerate {
        if let Some(basis_network) = provider.get_basis_network_by_lineage_and_subgraph_hash(
            &lineage,
            &subgraph_hash
        ).await? {
            return Ok(basis_network);
        }
    }






    let context = contexts.get(&read_lock!(graph).id).unwrap().clone();

    let mut graph_nodes = vec![context.graph_node.clone()];
    graph_nodes.extend(read_lock!(graph).children.clone());

    let json_nodes: Vec<crate::json_node::JsonNode> = graph_nodes
        .into_iter()
        .flat_map(|graph_node| {
            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let data_node = &context.data_node;
            let basis_node = {
                let lock = read_lock!(meta_context);
                lock.get_basis_node_by_lineage(&context.lineage)
                    .expect("Could not get basis node by lineage")
                    .unwrap()
            };
            basis_node
                .transformations
                .clone()
                .into_iter()
                .map(move |transformation| {
                    transformation
                        .transform(Arc::clone(&data_node))
                        .expect("Could not transform data node field")
                })
        })
        .collect();

    log::debug!("json_nodes count: {}", json_nodes.len());










    let maybe_network_transformation: Option<NetworkTransformation> = {
        if json_nodes.is_empty() {
            None
        } else {
            let json = context.generate_json_snippet(
                Arc::clone(&meta_context)
            )?;

            log::debug!("{}", json);

            let (network_transformation, (tokens)) = LLM::get_network_transformation(
                &subgraph_hash.to_string().unwrap(),
                &json,
                document_summary
            ).await?;

            stage_context.record_events("Network analysis", tokens);

            Some(network_transformation)
        }
    };

    let basis_network = BasisNetwork {
        id: ID::new(),
        description,
        subgraph_hash: subgraph_hash.clone(),
        lineage: lineage.clone(),
        network_transformation: maybe_network_transformation
    };

    provider
        .save_basis_network(&lineage, &subgraph_hash, basis_network.clone())
        .await?;

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
            log::debug!("unique subgraph: {}", &lock.subgraph_hash);
            unique_subgraphs.insert(lock.subgraph_hash.clone(), current.clone());
        }

        for child in &lock.children {
            queue.push_back(child.clone());
        }
    }

    log::debug!("Number of unique subgraphs: {:?}", unique_subgraphs.len());

    unique_subgraphs
}
