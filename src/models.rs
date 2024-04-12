use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub xpath: Option<String>,
    pub variants: Vec<String>,
    pub is_url: bool,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub hash: String,
    pub xml: String,
    pub data: Vec<NodeData>,
    pub children: Vec<Node>
}
