use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataStructure {
    pub tree: Option<NodeTreeDataStructure>,
    pub list: Option<NodeListDataStructure>,
    pub set: Option<NodeSetDataStructure>,
}

impl NodeDataStructure {
    pub fn is_atom(&self) -> bool {
        self.tree.is_none() && self.list.is_none() && self.set.is_none()
    }
}

pub enum TraversalDirection {
    Up,
    Sibling,
    Child,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchInstructions {
    traversal_direction: TraversalDirection,
    target_hash: String,
    target_attributes: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTreeDataStructure {
    root_node: SearchInstructions,
    parent_node: SearchInstructions,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeListDataStructure {
    next_node: SearchInstructions,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeSetDataStructure {
    next_node: SearchInstructions,
}

// provide element and ask llm which other element is the parent providing complete element opening tag
// search according to traversal direction and obtain node in output tree
// now we can get its hash and use that to generalise and obtain parent nodes

