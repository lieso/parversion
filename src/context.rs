use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Debug;
use xmltree::{Element, XMLNode};

use crate::id::{ID};
use crate::document::{DocumentNode};

struct Context {
    nodes: HashMap<ID, &XMLNode>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: XMLNode) -> ID {
        let id = ID::new();
        self.nodes.insert(id.clone(), &node);
        id
    }
}
