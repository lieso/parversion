use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::data_node::DataNode;
use crate::graph_node::{GraphNode, GraphNodeID};
use crate::document_node::DocumentNode;
use crate::meta_context::MetaContext;

pub type ContextID = ID;

#[derive(Clone, Debug)]
pub struct Context {
    pub id: ContextID,
    pub lineage: Lineage,
    pub document_node: Arc<RwLock<DocumentNode>>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
}

impl Context {
    pub fn generate_snippet(&self, meta_context: Arc<MetaContext>) -> String {
        log::trace!("In generate_snippet");

        let mut neighbour_ids = HashSet::new();
        let graph_node = self.graph_node.clone();
        
        Self::traverse_for_neighbours(
            Arc::clone(&graph_node),
            &mut neighbour_ids
        );

        let mut snippet = String::new();

        Self::traverse_for_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&graph_node),
            &neighbour_ids,
            &read_lock!(graph_node).id,
            &mut snippet
        );

        snippet
    }

    fn traverse_for_snippet(
        meta_context: Arc<MetaContext>,
        current_node: Arc<RwLock<GraphNode>>,
        neighbour_ids: &HashSet<GraphNodeID>,
        target_id: &GraphNodeID,
        snippet: &mut String,
    ) {
        let lock = read_lock!(current_node);
        let current_id = lock.id.clone();
        let current_context = meta_context.contexts.get(&current_id).unwrap();
        let document_node = current_context.document_node.clone();

        let should_render = if current_id == *target_id {
            let (mut a, b) = read_lock!(document_node).to_string_components();

            a = Self::mark_text(&a);
            snippet.push_str(&a);

            true
        } else if neighbour_ids.contains(&current_id) {
            let (a, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(&a);

            true
        } else {
            false
        };

        for child in &lock.children {
            Self::traverse_for_snippet(
                Arc::clone(&meta_context),
                Arc::clone(child),
                neighbour_ids,
                target_id,
                snippet,
            );
        }

        if should_render {
            let (_, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(b.as_deref().unwrap_or(""));
        }
    }

    fn traverse_for_neighbours(
        start_node: Arc<RwLock<GraphNode>>,
        visited: &mut HashSet<GraphNodeID>,
    ) {
        let mut queue: VecDeque<Arc<RwLock<GraphNode>>> = VecDeque::new();
        queue.push_back(Arc::clone(&start_node));

        while let Some(node) = queue.pop_front() {
            let lock = read_lock!(node);
            let graph_node_id = lock.id.clone();

            if visited.contains(&graph_node_id) {
                continue;
            }

            visited.insert(graph_node_id.clone());

            if visited.len() > 50 {
                return;
            }

            for child in lock.children.iter() {
                queue.push_back(Arc::clone(child));
            }

            for parent in lock.parents.iter() {
                queue.push_back(Arc::clone(parent));
            }
        }
    }

    fn mark_text(text: &str) -> String {
        let marker_prefix = "<!-- Target node: Start -->";
        let marker_suffix = "<!-- Target node: End -->";

        format!("{}{}{}", marker_prefix, text, marker_suffix)
    }
}
