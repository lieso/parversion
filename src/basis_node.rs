use serde::{Serialize, Deserialize};

use crate::transformation::{Transformation};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: String,
    pub hash: String,
    pub lineage: String,
    pub description: String,
    pub transformations: Vec<Transformation>,
}

impl BasisNode {
    pub fn new() -> self {

    }
}

impl BasisNode {
    pub fn apply_data_node_transformations(&self, data_node: DataNode) -> Value {

    }

    pub fn apply_node_transformations(&self, node: Graph<XmlNode>) -> Value {

    }
}


