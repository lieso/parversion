use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;
use crate::graph_node::GraphNode;
use crate::document_node::DocumentNode;
use crate::context::{Context, ContextID};
use crate::data_node::DataNode;

pub struct MetaContext {
    pub contexts: HashMap<ID, Arc<Context>>,
    pub document_root: Arc<RwLock<DocumentNode>>,
    pub graph_root: Arc<RwLock<GraphNode>>,
    pub data_nodes: HashMap<ID, Arc<DataNode>>,
}
