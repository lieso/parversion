use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};
use crate::graph_node::{GraphNodeData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub data: Arc<Mutex<Vec<NodeData>>>,
    pub structure: Arc<Mutex<Vec<NodeDataStructure>>>,
}

impl GraphNodeData for BasisNode {
    fn new() -> Self {
        BasisNode {
            data: Arc::new(Mutex::new(Vec::new())),
            structure: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
