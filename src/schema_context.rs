use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::{GraphNode};
use crate::schema_node::SchemaNode;

#[derive(Clone, Debug)]
pub struct SchemaContext {
    pub id: ID,
    pub lineage: Lineage,
    pub schema_node: Arc<SchemaNode>,
    pub graph_node: Arc<RwLock<GraphNode>>,
}
