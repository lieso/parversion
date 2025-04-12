use std::collections::{HashSet, HashMap, VecDeque};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio::task;
use futures::future;
use tokio::sync::Semaphore;
use futures::future::try_join_all;

use crate::prelude::*;
use crate::data_node::DataNode;
use crate::json_node::JsonNode;
use crate::basis_node::BasisNode;
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::transformation::{Transformation, HashTransformation};
use crate::provider::Provider;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::profile::Profile;
use crate::basis_network::{
    BasisNetwork,
    NetworkRelationship,
    Recursion
};
use crate::config::{CONFIG};
use crate::context::{Context, ContextID};
use crate::context_group::ContextGroup;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::transformation::{
    FieldTransformation,
    DataNodeFieldsTransform,
    Runtime
};

pub struct Analysis {
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn start<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<Self, Errors> {
        log::info!("Starting analysis...");



        // preparation

        let summary = meta_context.get_summary().await?;
        log::info!("Document summary: {}", summary);


        //





        let node_analysis = NodeAnalysis::new(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
        ).await?;

        log::info!("Completed node analysis");

        let network_analysis = NetworkAnalysis::new(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
        ).await?;

        log::info!("Completed network analysis");

        let analysis = Analysis {
            node_analysis,
            network_analysis,
        };

        Ok(analysis)
    }
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

impl NodeAnalysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<NodeAnalysis, Errors> {
        log::info!("Performing node analysis");

        let basis_nodes: Vec<BasisNode> = Self::get_basis_nodes(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
        ).await?;

        let node_analysis = NodeAnalysis {
            basis_nodes,
        };

        Ok(node_analysis)
    }

    async fn get_basis_nodes<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<Vec<BasisNode>, Errors> {
        log::trace!("In get_basis_nodes");

        let context_groups = ContextGroup::from_meta_context(Arc::clone(&meta_context));

        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

        if max_concurrency == 1 {
            let mut results = Vec::new();
            for context_group in context_groups {
                let cloned_provider = Arc::clone(&provider);
                let cloned_meta_context = Arc::clone(&meta_context);
                let result = Self::get_basis_node(
                    cloned_provider,
                    cloned_meta_context,
                    context_group.clone()
                ).await;
                results.push(result);
            }
            results.into_iter().collect::<Result<Vec<BasisNode>, Errors>>()
        } else {
            let semaphore = Arc::new(Semaphore::new(max_concurrency));
            let mut handles = Vec::new();

            for context_group in context_groups {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let cloned_provider = Arc::clone(&provider);
                let cloned_meta_context = Arc::clone(&meta_context);

                let handle = task::spawn(async move {
                    let _permit = permit; // Ensure permit is held until task completion
                    Self::get_basis_node(
                        cloned_provider,
                        cloned_meta_context,
                        context_group.clone()
                    ).await
                });
                handles.push(handle);
            }

            let results: Vec<Result<BasisNode, Errors>> = try_join_all(handles).await?;
            results.into_iter().collect::<Result<Vec<BasisNode>, Errors>>()
        }
    }

    async fn get_basis_node<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        context_group: ContextGroup,
    ) -> Result<BasisNode, Errors> {
        log::trace!("In get_basis_node");

        let lineage = &context_group.lineage.clone();
        let data_node = &context_group.contexts.first().unwrap().data_node.clone();
        let hash = data_node.hash.clone();
        let description = data_node.description.clone();

        if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
            log::info!("Provider has supplied basis node");

            return Ok(basis_node);
        };

        let field_transformations: Vec<FieldTransformation> = LLM::get_field_transformations(
            Arc::clone(&meta_context),
            context_group.clone()
        ).await?;

        log::info!("Obtained field transformation");

        let basis_node = BasisNode {
            id: ID::new(),
            hash,
            description,
            lineage: lineage.clone(),
            transformations: field_transformations,
        };

        provider.save_basis_node(
            &lineage,
            basis_node.clone(),
        ).await?;

        Ok(basis_node)
    }
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

impl NetworkAnalysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<NetworkAnalysis, Errors> {
        log::info!("Performing network analysis");

        let basis_networks: Vec<BasisNetwork> = Self::get_basis_networks(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
        ).await?;

        let network_analysis = NetworkAnalysis {
            basis_networks,
        };

        Ok(network_analysis)
    }

    async fn get_basis_networks<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<Vec<BasisNetwork>, Errors> {
        log::trace!("In get_basis_networks");

        let graph_root = Arc::clone(&meta_context.graph_root);

        let mut queue = VecDeque::new();
        let mut unique_subgraphs = HashMap::new();

        queue.push_back(graph_root);

        while let Some(current) = queue.pop_front() {
            let current_read = read_lock!(current);

            log::info!("graph_node: {}", current_read.description);
            log::info!("subgraph_hash: {}", current_read.subgraph_hash);

            if current_read.children.is_empty() {
                log::info!("Current node is leaf node. Not proceeding further.");
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

        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

        if max_concurrency == 1 {
            let mut results = Vec::new();
            for subgraph in unique_subgraphs.values().cloned() {
                let cloned_provider = Arc::clone(&provider);
                let cloned_meta_context = Arc::clone(&meta_context);
                if let Some(result) = Self::get_basis_network(cloned_provider, cloned_meta_context, subgraph.clone()).await? {
                    results.push(result);
                }
            }

            Ok(results.into_iter().collect::<Vec<BasisNetwork>>())
        } else {
            let semaphore = Arc::new(Semaphore::new(max_concurrency));
            let mut handles = Vec::new();
            for subgraph in unique_subgraphs.values().cloned() {
                let _permit = semaphore.clone().acquire_owned().await.unwrap();
                let cloned_provider = Arc::clone(&provider);
                let cloned_meta_context = Arc::clone(&meta_context);

                let handle = task::spawn(async move {
                    Self::get_basis_network(
                        cloned_provider,
                        cloned_meta_context,
                        subgraph.clone()
                    ).await
                });
                handles.push(handle);
            }
            let results: Vec<Result<Option<BasisNetwork>, Errors>> = try_join_all(handles).await?;

            let networks: Vec<BasisNetwork> = results.into_iter()
                .filter_map(|res| match res {
                    Ok(Some(network)) => Some(network),
                    _ => None,
                })
                .collect();

            Ok(networks)
        }
    }

    async fn get_basis_network<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        graph: Graph
    ) -> Result<Option<BasisNetwork>, Errors> {
        log::trace!("In get_basis_network");


        let current_context = meta_context.contexts
            .get(&read_lock!(graph).id)
            .unwrap()
            .clone();
        
        let current_json = current_context.generate_json(
            Arc::clone(&provider),
            Arc::clone(&meta_context)
        ).await?;

        let target_subgraph_hash = read_lock!(current_context.graph_node).subgraph_hash().to_string().unwrap();



        let overall_context = meta_context.get_summary().await?;

        



        if !current_json.is_empty() {


            if read_lock!(graph).parents.is_empty() {


                log::info!("Skipping root node");



            } else {


                let parent: Graph = read_lock!(graph).parents
                    .first()
                    .unwrap()
                    .clone();
                let sibling_contexts: Vec<_> = read_lock!(parent).children
                    .iter()
                    .map(|sibling| {
                        meta_context.contexts
                            .get(&read_lock!(sibling).id)
                            .unwrap()
                            .clone()
                    })
                    .collect();


                let mut sibling_jsons = Vec::new();


                for sibling_context in sibling_contexts {
                    let sibling_json = sibling_context.generate_json(
                        Arc::clone(&provider),
                        Arc::clone(&meta_context)
                    ).await?;

                    log::debug!("sibling_json: {:?}", sibling_json);

                    let subgraph_hash: String = read_lock!(sibling_context.graph_node).subgraph_hash().to_string().unwrap();

                    if !sibling_json.is_empty() {
                        sibling_jsons.push((subgraph_hash, sibling_json));
                    }
                }

                log::debug!("current_json: {:?}", current_json);


                if sibling_jsons.len() > 1 {

                    let matches = LLM::get_relationships(
                        overall_context.clone(),
                        target_subgraph_hash.clone(),
                        sibling_jsons.clone()
                    ).await?;



                    if matches.len() > 0 {

                        let associated_subgraphs = matches.iter().cloned()
                            .chain(std::iter::once(target_subgraph_hash.clone()))
                            .collect();

                        let basis_network = BasisNetwork {
                            id: ID::new(),
                            description: "Placeholder Description".to_string(),
                            relationship: NetworkRelationship::Association(associated_subgraphs),
                        };

                        return Ok(Some(basis_network));
                    }



                }


            }

        }





        let basis_network = BasisNetwork {
            id: ID::new(),
            description: "Placeholder Description".to_string(),
            relationship: NetworkRelationship::Recursion(Recursion {
                lineage: Lineage::new(),
                transformation: DataNodeFieldsTransform {
                    id: ID::new(),
                    runtime: Runtime::QuickJS,
                    code: "".to_string(),
                },
            }),
        };

        Ok(None)
    }
}
