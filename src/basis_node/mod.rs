use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};
use crate::graph_node::{GraphNodeData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub data: Arc<RwLock<Vec<NodeData>>>,
    pub structure: Arc<RwLock<Vec<NodeDataStructure>>>,
}

impl GraphNodeData for BasisNode {
    fn new() -> Self {
        BasisNode {
            data: Arc::new(RwLock::new(Vec::new())),
            structure: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
