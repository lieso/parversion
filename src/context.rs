use std::collections::HashMap;

use crate::id::{ID};

pub struct Context {
    nodes: HashMap<ID, String>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: String) -> ID {
        let id = ID::new();
        self.nodes.insert(id.clone(), node);
        id
    }
}
