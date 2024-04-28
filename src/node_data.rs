use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataValue {
    //pub is_url: bool,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    //pub select: Box<dyn Fn(&str) -> Option<NodeDataValue>>,
    pub xpath: String,
    pub name: String,
    pub value: Option<NodeDataValue>,
}
