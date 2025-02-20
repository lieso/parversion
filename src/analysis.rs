use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::task;
use futures::future;
use tokio::sync::Semaphore;

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

        unimplemented!()

    }
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

