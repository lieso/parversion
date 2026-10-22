use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::graph_node::Graph;
use crate::normal_context::NormalContext;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct NormalMetaContext {
    pub contexts: HashMap<ID, Arc<NormalContext>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<NormalContext>>,
}

impl NormalMetaContext {
    // Warning: this assumes both NormalMetaContexts are already part of the same graph
    pub fn merge(mut self, other: NormalMetaContext) -> Result<NormalMetaContext, Errors> {
        self.contexts.extend(other.contexts);
        self.contexts_lookup.extend(other.contexts_lookup);
        Ok(self)
    }

    pub fn relineage(&self) {
        self.relineage_graph_node(
            Arc::clone(&self.graph_root),
            Lineage::new()
        );
    }

    fn relineage_graph_node(&self, graph_node: Graph, parent_lineage: Lineage) {
        if let Some(context) = self.contexts_lookup.get(&read_lock!(graph_node).id) {
            let data_node = &context.data_node;
            let hash = data_node.generate_hash();
            let lineage = parent_lineage.with_hash(hash.clone());

            {
                let mut lock = write_lock!(graph_node);
                lock.hash = hash;
                lock.lineage = lineage.clone();
                lock.acyclic_lineage = lineage.acyclic();
                lock.indexed_lineages = Arc::new(RwLock::new(HashMap::new()));
            }

            for child in &read_lock!(graph_node).children {
                self.relineage_graph_node(
                    Arc::clone(&child),
                    lineage.clone()
                );
            }
        } else {
            {
                let mut lock = write_lock!(graph_node);
                lock.hash = Hash::new();
                lock.lineage = Lineage::new();
                lock.acyclic_lineage = Lineage::new();
                lock.indexed_lineages = Arc::new(RwLock::new(HashMap::new()));
            }

            for child in &read_lock!(graph_node).children {
                self.relineage_graph_node(
                    Arc::clone(&child),
                    parent_lineage.clone()
                );
            }
        }
    }
}
