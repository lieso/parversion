use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode {
    pub id: ID,
    pub description: String,
    pub context_id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub children: Vec<Arc<RwLock<GraphNode>>>,
}

pub type Graph = Arc<RwLock<GraphNode>>;

impl GraphNode {
    pub fn from_data_node(data_node: &DataNode) -> Self {
        GraphNode {
            id: ID::new(),
            context_id: data_node.context_id.clone(),
            description: data_node.description.clone(),
            hash: data_node.hash.clone(),
            lineage: data_node.lineage.clone(),
            children: Vec::new(),
        }
    }
}
