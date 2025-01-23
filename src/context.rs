use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::analysis::{KeyID, Dataset, GraphNodeID};
use crate::graph_node::{GraphNode};

pub struct Context {}

impl Context {

    pub fn generate_snippet(
        dataset: Arc<Dataset>,
        key: &KeyID
    ) -> String {

        let root_graph_node: Arc<RwLock<GraphNode>> = dataset.graph_nodes.get(
            &dataset.root_node_key_id
        ).unwrap().clone();

        let graph_node = dataset.graph_nodes.get(key).clone().unwrap();
        let graph_node_id = read_lock!(graph_node).id.clone();




        let mut neighbour_ids = HashSet::new();
        Self::traverse_for_neighbours(Arc::clone(graph_node), &mut neighbour_ids);




        let mut snippet = String::new();

        Self::traverse_for_snippet(
            Arc::clone(&dataset),
            Arc::clone(&root_graph_node),
            &mut snippet,
            &neighbour_ids,
            &graph_node_id
        );

        log::debug!("-----------------------------------------------------------------------------------------------------");
        log::debug!("snippet: {}", snippet);
        log::debug!("-----------------------------------------------------------------------------------------------------");



        unimplemented!()
    }

    fn traverse_for_snippet(
        dataset: Arc<Dataset>,
        current_node: Arc<RwLock<GraphNode>>,
        snippet: &mut String,
        neighbour_ids: &HashSet<GraphNodeID>,
        target_id: &GraphNodeID,
    ) {
        let lock = read_lock!(current_node);
        let current_id = lock.id.clone();

        let should_render = if current_id == *target_id {
            let key_id = dataset.graph_key.get(&current_id).unwrap();
            let document_node = dataset.document_nodes.get(&key_id).unwrap();
            let (mut a, b) = read_lock!(document_node).to_string_components();

            a = Self::mark_text(&a);
            snippet.push_str(&a);

            true
        } else if neighbour_ids.contains(&current_id) {
            let key_id = dataset.graph_key.get(&current_id).unwrap();
            let document_node = dataset.document_nodes.get(&key_id).unwrap();
            let (a, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(&a);

            true
        } else {
            false
        };

        if should_render {
            let key_id = dataset.graph_key.get(&current_id).unwrap();
            let document_node = dataset.document_nodes.get(&key_id).unwrap();
            let (_, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(b.as_deref().unwrap_or(""));
        }

        for child in &lock.children {
            Self::traverse_for_snippet(
                Arc::clone(&dataset),
                Arc::clone(child),
                snippet,
                neighbour_ids,
                target_id,
            );
        }
    }

    fn mark_text(text: &str) -> String {
        let marker_prefix = "<!-- Target node: Start -->";
        let marker_suffix = "<!-- Target node: End -->";

        format!("{}{}{}", marker_prefix, text, marker_suffix)
    }

    fn traverse_for_neighbours(
        start_node: Arc<RwLock<GraphNode>>,
        visited: &mut HashSet<GraphNodeID>,
    ) {
        let mut stack = VecDeque::new();
        stack.push_back(Arc::clone(&start_node));

        while let Some(node) = stack.pop_back() {
            let lock = read_lock!(node);
            let graph_node_id = lock.id.clone();

            if visited.contains(&graph_node_id) {
                continue;
            }

            visited.insert(graph_node_id.clone());

            if visited.len() > 20 {
                return;
            }

            for child in lock.children.iter() {
                stack.push_back(Arc::clone(child));
            }

            for parent in lock.parents.iter() {
                stack.push_back(Arc::clone(parent));
            }
        }
    }
}
