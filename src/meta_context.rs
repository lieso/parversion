use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::graph_node::{Graph, GraphNode};
use crate::context::{Context};
use crate::llm::LLM;
use crate::interface::Interface;

pub struct MetaContext {
    pub contexts: HashMap<ID, Arc<Context>>,
    pub graph_root: Arc<RwLock<GraphNode>>,
    pub interface: Arc<Interface>,
}

impl MetaContext {
    pub fn get_original_document(&self) -> String {
        log::trace!("In get_original_document");

        let mut document = String::new();
        let mut visited_lineages: HashSet<Lineage> = HashSet::new();
        let root_node = self.graph_root.clone();

        traverse_for_condensed_document(
            self,
            Arc::clone(&root_node),
            &mut visited_lineages,
            &mut document
        );

        document
    }
}

fn traverse_for_condensed_document(
    meta_context: &MetaContext,
    current_node: Graph,
    visited_lineages: &mut HashSet<Lineage>,
    document: &mut String
) {
    let lock = read_lock!(current_node);
    let current_id = lock.id.clone();
    let current_context = meta_context.contexts.get(&current_id).unwrap();
    let current_lineage = current_context.lineage.clone();
    let document_node = current_context.document_node.clone();

    let should_render = !visited_lineages.contains(&current_lineage);

    visited_lineages.insert(current_lineage.clone());

    if should_render {
        let (a, _) = read_lock!(document_node).to_string_components();

        document.push_str(&a);
    }

    for child in &lock.children {
        traverse_for_condensed_document(
            &meta_context,
            Arc::clone(child),
            visited_lineages,
            document
        );
    }

    if should_render {
        let (_, b) = read_lock!(document_node).to_string_components();

        document.push_str(b.as_deref().unwrap_or(""));
    }
}
