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
use crate::context::Context;
use crate::llm::LLM;

pub struct Analysis {
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        meta_context: MetaContext,
        contexts: HashMap<ContextID, Arc<Context>>
    ) -> Result<Self, Errors> {
        unimplemented!()
    }
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

