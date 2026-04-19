use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::HashSet;

use crate::prelude::*;
use crate::xpath::XPath;
use crate::graph_node::{Graph, GraphNode};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TraversalValue {
    pub name: String,
    pub xpath: XPath,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub candidate: XPath,
    pub parent_values: Vec<TraversalValue>,
    pub candidate_values: Vec<TraversalValue>,
    pub filter_function: String,
}

pub fn get_original_document_condensed(meta_context: Arc<RwLock<MetaContext>>) -> Result<String, Errors> {
    let mut document = String::new();
    let mut visited_lineages: HashSet<Lineage> = HashSet::new();
    let root_node = {
        let lock = read_lock!(meta_context);
        lock.graph_root.clone().unwrap()
    };

    traverse_for_condensed_document(
        Arc::clone(&meta_context),
        Arc::clone(&root_node),
        &mut visited_lineages,
        &mut document,
    );

    Ok(document)
}

fn traverse_for_condensed_document(
    meta_context: Arc<RwLock<MetaContext>>,
    current_node: Graph,
    visited_lineages: &mut HashSet<Lineage>,
    document: &mut String,
) {
    let lock = read_lock!(current_node);
    let current_id = lock.id.clone();
    let current_context = read_lock!(meta_context).contexts.clone().unwrap();
    let current_context = current_context.get(&current_id).unwrap();
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
            meta_context.clone(),
            Arc::clone(child),
            visited_lineages,
            document,
        );
    }

    if should_render {
        let (_, b) = read_lock!(document_node).to_string_components();

        document.push_str(b.as_deref().unwrap_or(""));
    }
}
