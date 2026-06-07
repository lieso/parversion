use std::collections::{HashSet, HashMap};
use std::sync::Arc;

use crate::prelude::*;
use crate::context::Context;
use crate::graph_node::Graph;
use crate::translation_node::TranslationNode;

pub struct TranslationContext {
    pub input_contexts: Option<HashMap<NodeID, Arc<Context>>>,
    pub input_graph_root: Option<Graph>,
    pub target_contexts: Option<HashMap<NodeID, Arc<Context>>>,
    pub target_graph_root: Option<Graph>,
    pub translation_nodes: Option<HashMap<ID, Arc<TranslationNode>>>,
}

impl TranslationContext {
    pub fn new() -> Self {
        Self {
            input_contexts: None,
            input_graph_root: None,
            target_contexts: None,
            target_graph_root: None,
            translation_nodes: None,
        }
    }

    pub fn must_get_unique_input_contexts(&self) -> Result<Vec<Arc<Context>>, Errors> {
        Self::unique_contexts_from(&self.input_contexts)
    }

    pub fn must_get_unique_target_contexts(&self) -> Result<Vec<Arc<Context>>, Errors> {
        Self::unique_contexts_from(&self.target_contexts)
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
    
    pub fn update_translation_nodes(&mut self, nodes: HashMap<ID, Arc<TranslationNode>>) {
        self.translation_nodes = Some(nodes);
    }

    fn unique_contexts_from(maybe_contexts: &Option<HashMap<ID, Arc<Context>>>) -> Result<Vec<Arc<Context>>, Errors> {
        let contexts = maybe_contexts.as_ref().ok_or_else(|| {
            Errors::DeficientNormalizationContextError("Contexts missing in TranslationContext".to_string())
        })?;

        let mut seen = HashSet::new();

        let unique_contexts = contexts
            .values()
            .filter(|c| !c.data_node.fields.is_empty())
            .filter(|c| {
                seen.insert(c.id.to_string()) && seen.insert(c.lineage.to_string())
            })
            .cloned()
            .collect();

        Ok(unique_contexts)
    }
}
