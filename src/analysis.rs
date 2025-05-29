use std::collections::{HashMap, VecDeque};
use std::sync::{Arc};
use tokio::task;
use tokio::sync::Semaphore;
use futures::future::try_join_all;

use crate::prelude::*;
use crate::basis_node::BasisNode;
use crate::provider::Provider;
use crate::graph_node::{Graph};
use crate::basis_network::{
    BasisNetwork,
    NetworkRelationship,
};
use crate::config::{CONFIG};
use crate::context_group::ContextGroup;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::transformation::{
    FieldTransformation,
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
                    let _permit = permit;
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
        _meta_context: Arc<MetaContext>,
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

        let basis_networks: Vec<BasisNetwork> = Self::generate_basis_networks(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
        ).await?;

        let network_analysis = NetworkAnalysis {
            basis_networks,
        };

        Ok(network_analysis)
    }

    async fn generate_basis_networks<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
    ) -> Result<Vec<BasisNetwork>, Errors> {
        log::trace!("In generate_basis_networks");

        let graph_root = Arc::clone(&meta_context.graph_root);

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

        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

        if max_concurrency == 1 {
            let mut results = Vec::new();
            for subgraph in unique_subgraphs.values().cloned() {
                let cloned_provider = Arc::clone(&provider);
                let cloned_meta_context = Arc::clone(&meta_context);
                let result = Self::generate_basis_network(cloned_provider, cloned_meta_context, subgraph.clone()).await?;
                results.push(result);
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
                    Self::generate_basis_network(
                        cloned_provider,
                        cloned_meta_context,
                        subgraph.clone()
                    ).await
                });
                handles.push(handle);
            }
            let results: Vec<Result<BasisNetwork, Errors>> = try_join_all(handles).await?;
            results.into_iter().collect::<Result<Vec<BasisNetwork>, Errors>>()
        }
    }

    async fn generate_basis_network<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        graph: Graph
    ) -> Result<BasisNetwork, Errors> {
        log::trace!("In generate_basis_network");

        let target_subgraph_hash = read_lock!(graph).subgraph_hash.clone();
        log::debug!("target_subgraph_hash: {}", target_subgraph_hash.to_string().unwrap());

        if read_lock!(graph).parents.is_empty() {
            log::info!("Node is root node; going to create null network for this node");

            return Ok(Self::add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
        }

        let current_context = meta_context.contexts
            .get(&read_lock!(graph).id)
            .unwrap()
            .clone();

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
                meta_context.contexts
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

                return Ok(Self::add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
            }

            if !sibling_json.is_empty() {
                sibling_jsons.push((subgraph_hash.to_string().unwrap().clone(), sibling_json));
            }
        }

        log::info!("Completed processing all sibling contexts.");

        if sibling_jsons.len() <= 1 {
            log::info!("Only one subgraph contains meaningful JSON; we will not investigate it further.");

            return Ok(Self::add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
        }

        log::info!("Going to consult LLM for relationships between subgraphs...");

        let overall_context = meta_context.get_summary().await?;

        let (name, matches) = LLM::get_relationships(
            overall_context.clone(),
            target_subgraph_hash.to_string().unwrap().clone(),
            sibling_jsons.clone()
        ).await?;

        log::info!("Done asking LLM for relationships");

        if matches.is_empty() {
            log::info!("LLM did not find any relationships between subgraphs");

            return Ok(Self::add_null_network(provider.clone(), target_subgraph_hash.clone()).await?);
        }

        log::info!("LLM determined subgraphs are associated: {:?}", matches);

        let associated_subgraphs = matches.iter().cloned()
            //.chain(std::iter::once(target_subgraph_hash.to_string().unwrap().clone()))
            .collect();

        let basis_network = BasisNetwork {
            id: ID::new(),
            description: "Placeholder Description".to_string(),
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
}
