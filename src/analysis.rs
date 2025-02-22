use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::task;
use futures::future;
use tokio::sync::Semaphore;
use futures::future::try_join_all;

use crate::prelude::*;
use crate::data_node::DataNode;
use crate::json_node::JsonNode;
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::transformation::{Transformation, HashTransformation};
use crate::provider::Provider;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::profile::Profile;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::config::{CONFIG};
use crate::context::{Context, ContextID};
use crate::llm::LLM;
use crate::meta_context::MetaContext;

pub struct Analysis {
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn start<P: Provider>(
        provider: Arc<P>,
        meta_context: MetaContext,
        contexts: HashMap<ContextID, Arc<Context>>
    ) -> Result<Self, Errors> {

        let meta_context = Arc::new(meta_context);
        let contexts = Arc::new(contexts);




        let node_analysis = NodeAnalysis::new(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
            Arc::clone(&contexts)
        ).await?;





        unimplemented!()
    }
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

impl NodeAnalysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        contexts: Arc<HashMap<ContextID, Arc<Context>>>
    ) -> Result<NodeAnalysis, Errors> {

        log::info!("Performing node analysis");


        let basis_nodes: Vec<BasisNode> = Self::get_basis_nodes(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
            Arc::clone(&contexts),
        ).await?;



        let node_analysis = NodeAnalysis {
            basis_nodes,
        };

        Ok(node_analysis)
    }

    async fn get_basis_nodes<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        contexts: Arc<HashMap<ContextID, Arc<Context>>>
    ) -> Result<Vec<BasisNode>, Errors> {
        log::trace!("In get_basis_nodes");

        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        let mut context_groups: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

        for context in contexts.values() {
            context_groups.entry(context.lineage.clone())
                .or_insert_with(Vec::new)
                .push(context.clone());
        }

        let mut handles = Vec::new();
        for (lineage, context_group) in context_groups {
            let _permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let cloned_lineage = lineage.clone();

            let handle = task::spawn(async move {
                Self::get_basis_node(
                    cloned_provider,
                    cloned_meta_context,
                    lineage,
                    context_group.clone()
                ).await
            });
            handles.push(handle);
        }

        let results: Vec<Result<BasisNode, Errors>> = try_join_all(handles).await?;

        results.into_iter().collect::<Result<Vec<BasisNode>, Errors>>()
    }

    async fn get_basis_node<P: Provider>(
        provider: Arc<P>,
        meta_context: Arc<MetaContext>,
        lineage: Lineage,
        context_group: Vec<Arc<Context>>,
    ) -> Result<BasisNode, Errors> {
        log::trace!("In get_basis_node");

        unimplemented!()
    }
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

