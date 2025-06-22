use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::graph_node::{GraphNode, GraphNodeID};

#[derive(Clone, Debug)]
pub struct SchemaContext {
    pub id: ID,
    pub lineage: Lineage,
    pub schema_node: Arc<SchemaNode>,
    pub graph_node: Arc<RwLock<GraphNode>>,
}

impl SchemaContext {
    pub fn generate_snippet(&self, meta_context: Arc<RwLock<MetaContext>>) -> String {
        log::trace!("In generate_snippet");

        let mut neighbour_ids = HashSet::new();
        let graph_node = self.graph_node.clone();

        Self::traverse_for_neighbours(
            Arc::clone(&graph_node),
            &mut neighbour_ids
        );

        let mut snippet = String::new();
        let lock = read_lock!(meta_context);
        let graph_root = lock.normal_schema_graph_root.clone().unwrap();

        unimplemented!()
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

            if visited.len() > 20 {
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
}
