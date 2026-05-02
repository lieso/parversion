use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::GraphNode;
use crate::data_node::DataNode;

#[derive(Clone, Debug)]
pub struct NormalContext {
    pub id: ID,
    pub lineage: Lineage,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
}
