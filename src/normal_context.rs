use std::sync::{Arc, RwLock};

use crate::context::Context;
use crate::data_node::DataNode;
use crate::graph_node::GraphNode;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct NormalContext {
    pub id: ID,
    pub network_name: Option<String>,
    pub network_description: Option<String>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
    pub contexts: Vec<Arc<Context>>,
}
