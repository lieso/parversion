use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub data: Arc<Mutex<Vec<NodeData>>>,
    pub structure: Arc<Mutex<Vec<NodeDataStructure>>>,
}
