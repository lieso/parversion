use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<Mutex<BasisNode>>>,
    pub children: Vec<Arc<Mutex<BasisNode>>>,
    pub data: Arc<Mutex<Vec<NodeData>>>,
    pub structure: Arc<Mutex<Vec<NodeDataStructure>>>,
}
