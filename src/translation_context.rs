use std::collections::HashMap;
use std::sync::Arc;

use crate::prelude::*;
use crate::context::Context;
use crate::graph_node::Graph;

pub struct TranslationContext {
    pub input_contexts: Option<HashMap<ID, Arc<Context>>>,
    pub input_graph_root: Option<Graph>,
    pub target_contexts: Option<HashMap<ID, Arc<Context>>>,
    pub target_graph_root: Option<Graph>,
}

impl TranslationContext {
    pub fn new() -> Self {
        Self {
            input_contexts: None,
            input_graph_root: None,
            target_contexts: None,
            target_graph_root: None,
        }
    }

    pub fn update_contexts(
        &mut self,
        input_contexts: HashMap<ID, Arc<Context>>,
        input_graph_root: Graph,
        target_contexts: HashMap<ID, Arc<Context>>,
        target_graph_root: Graph
    ) {
        self.input_contexts = Some(input_contexts);
        self.input_graph_root = Some(input_graph_root);
        self.target_contexts = Some(target_contexts);
        self.target_graph_root = Some(target_graph_root);
    }
}
