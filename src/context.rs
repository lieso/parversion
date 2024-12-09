use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Debug;

impl ID {
    fn new() -> Self {
        ID {}
    }
}

struct Context<N> {
    nodes: HashMap<ID, N>,
}

impl<N> Context<N> {
    pub fn new() -> Self {
        Context {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: N) -> ID {
        let id = ID::new();
        self.nodes.insert(id.clone(), node);
        id
    }
}
