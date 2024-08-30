use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};
use crate::graph_node::{GraphNodeData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub data: Arc<RwLock<Vec<NodeData>>>,
    pub structure: Arc<RwLock<Vec<NodeDataStructure>>>,
    pub description: String,
}

impl GraphNodeData for BasisNode {
    fn new(description: String) -> Self {
        BasisNode {
            data: Arc::new(RwLock::new(Vec::new())),
            structure: Arc::new(RwLock::new(Vec::new())),
            description,
        }
    }

    fn describe(&self) -> String {
        self.description.to_string()
    }
}
