use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub xpath: String,
    pub key: String,
    pub is_url: bool,
    pub value: Option<String>,
}

impl NodeData {
    pub fn to_tuple(self) -> (String, String) {
        let value = self.value.unwrap();

        (self.key, value)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub hash: String,
    pub xml: String,
    pub data: Vec<NodeData>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn to_hash_set(&self) -> HashSet<(String, String)> {
        self.data
            .iter()
            .cloned()
            .map(|data| (data.key, data.value.unwrap()))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexObject {
    pub id: String,
    pub type_id: String,
    pub set: HashSet<(String, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexType {
    pub id: String,
    pub set: HashSet<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub node_complex_object: HashMap<String, ComplexObject>,
    pub complex_types: Vec<ComplexType>,
    pub complex_objects: HashMap<String, Vec<ComplexObject>>,
}
