use serde::{Serialize, Deserialize};

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
pub struct NodeTreeDataStructure {
    traversal_direction: TraversalDirection,
    parent_hash: String,
}

// provide element and ask llm which other element is the parent providing complete element opening tag
// search according to traversal direction and obtain node in output tree
// now we can get its hash and use that to generalise and obtain parent nodes

