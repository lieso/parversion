use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::DataNode;

pub type Graph = Arc<RwLock<GraphNode>>;

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
    pub fn from_data_node(data_node: Arc<RwLock<DataNode>>, parents: Vec<Graph>) -> Self {
        let lock = read_lock!(data_node);

        GraphNode {
            id: ID::new(),
            parents,
            description: lock.description.clone(),
            hash: lock.hash.clone(),
            lineage: lock.lineage.clone(),
            children: Vec::new(),
        }
    }
}
