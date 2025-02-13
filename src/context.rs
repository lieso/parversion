use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::DataNode;
use crate::graph_node::GraphNode;
use crate::document_node::DocumentNode;

pub type ContextID = ID;

pub struct Context {
    pub id: ContextID,
    pub document_node: Arc<RwLock<DocumentNode>>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
}
