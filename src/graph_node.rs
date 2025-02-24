use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::DataNode;

pub type Graph = Arc<RwLock<GraphNode>>;
pub type GraphNodeID = ID;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode {
    pub id: ID,
    pub parents: Vec<Graph>,
    pub description: String,
    pub hash: Hash,
    pub lineage: Lineage,
    pub children: Vec<Graph>,
}

impl GraphNode {
    pub fn from_data_node(data_node: Arc<DataNode>, parents: Vec<Graph>) -> Self {
        GraphNode {
            id: ID::new(),
            parents,
            description: data_node.description.clone(),
            hash: data_node.hash.clone(),
            lineage: data_node.lineage.clone(),
            children: Vec::new(),
        }
    }
}
