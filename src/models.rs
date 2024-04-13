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
    pub fn to_hash_map(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(self.key, self.value);
        
        map
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub hash: String,
    pub xml: String,
    pub data: Vec<NodeData>,
    pub children: Vec<Node>,
    pub complex_object_id: Option<String>,
}

impl Node {
    pub fn to_hash_set(&self) -> HashSet<HashMap<String, String>> {
        let mut set = HashSet::new();

        for item in &self.data.iter() {
            set.insert(
                item.to_hash_map()
            );
        }

        set
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexObject {

}
