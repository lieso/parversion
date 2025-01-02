use std::collections::HashMap;

use crate::id::{ID};
use crate::document_node::DocumentNode;

pub struct Context {
    nodes: HashMap<ID, DocumentNode>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: &DocumentNode) -> ID {
        let id = ID::new();
        self.nodes.insert(id.clone(), node.clone());
        id
    }
}
