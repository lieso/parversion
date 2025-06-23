use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::graph_node::{Graph, GraphNode, GraphNodeID};

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

        let lock = read_lock!(meta_context);
        let graph_root = lock.normal_schema_graph_root.clone().unwrap();

        let snippet = Self::traverse_for_snippet(
            Arc::clone(&meta_context),
            Arc::clone(&graph_root),
            &neighbour_ids,
            &read_lock!(graph_node).id,
        );

        format!("{{ {} }}", snippet)
    }

    fn traverse_for_snippet(
        meta_context: Arc<RwLock<MetaContext>>,
        current_node: Graph,
        neighbour_ids: &HashSet<GraphNodeID>,
        target_id: &GraphNodeID,
    ) -> String {
        let (is_neighbour, is_target) = {
            let lock = read_lock!(current_node);
            let id = &lock.id;

            (
                neighbour_ids.contains(id),
                *id == *target_id,
            )
        };
        let schema_node = {
            let lock = read_lock!(meta_context);
            let normal_schema_contexts = lock.normal_schema_contexts
                .as_ref()
                .unwrap();
            let schema_context = normal_schema_contexts
                .get(&read_lock!(current_node).id)
                .unwrap();
            Arc::clone(&schema_context.schema_node)
        };

        let children = {
            let lock = read_lock!(current_node);
            lock.children.clone()
        };
        let inner_schema: Vec<String> = children
            .iter()
            .map(|child| {

                Self::traverse_for_snippet(
                    Arc::clone(&meta_context),
                    Arc::clone(child),
                    neighbour_ids,
                    target_id,
                )

            })
            .filter(|item| !item.is_empty())
            .collect();

        let json_schema_key: String = {
            if is_target {
                format!(r#"START TARGET SCHEMA KEY >>>{}<<< END TARGET SCHEMA KEY"#, schema_node.name)
            } else {
                schema_node.name.clone()
            }
        };

        if is_neighbour || is_target {
            if schema_node.data_type == "array" {
                format!(r#""{}": {{ "description": "{}", "data_type": "array", "items": {{ {} }} }}"#,
                    json_schema_key,
                    schema_node.description,
                    inner_schema.join(", ")
                )
            } else if schema_node.data_type == "object" {
                format!(r#""{}": {{ "description": "{}", "data_type": "object", "properties": {{ {} }} }}"#,
                    json_schema_key,
                    schema_node.description,
                    inner_schema.join(", ")
                )
            } else {
                format!(r#""{}": {{ "description": "{}", "data_type": "{}" }}"#,
                    json_schema_key,
                    schema_node.description,
                    schema_node.data_type
                )
            }
        } else {
            inner_schema.join(", ")
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
