use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;
use crate::graph_node::GraphNode;
use crate::document_node::DocumentNode;
use crate::context::ContextID;

pub struct MetaContext {
    pub context_ids: HashMap<ID, ContextID>,
    pub document_root: Arc<RwLock<DocumentNode>>,
    pub graph_root: Arc<RwLock<GraphNode>>,
}
