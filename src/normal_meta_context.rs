use std::sync::Arc;
use std::collections::{HashMap, VecDeque};

use crate::prelude::*;
use crate::graph_node::Graph;
use crate::normal_context::NormalContext;

#[derive(Clone, Debug)]
pub struct NormalMetaContext {
    pub contexts: HashMap<ID, Arc<NormalContext>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<NormalContext>>,
}

impl NormalMetaContext {
    pub fn collect_contexts_by_network_name(
        &self,
        network_name: &str
    ) -> Result<Vec<Arc<Context>>, Errors> {
        let mut contexts: Vec<Arc<Context>> = Vec::new();

        let mut queue: VecDeque<Graph> = VecDeque::new();
        queue.push_back(Arc::clone(&self.graph_root));

        while let Some(node) = queue.pop_front() {
            let normal_context = self.contexts_lookup
                .get(&read_lock!(node).id)
                .cloned()
                .unwrap();

            if let Some(node_network_name) = &normal_context.network_name {
                if node_network_name == network_name {
                    contexts.extend_from_slice(&normal_context.contexts);
                }
            }

            for child in &read_lock!(node).children {
                queue.push_back(child.clone());
            }
        }

        Ok(contexts)
    }

    // Warning: this assumes both NormalMetaContexts are already part of the same graph
    pub fn merge(mut self, other: NormalMetaContext) -> Result<NormalMetaContext, Errors> {
        self.contexts.extend(other.contexts);
        self.contexts_lookup.extend(other.contexts_lookup);
        Ok(self)
    }
}
