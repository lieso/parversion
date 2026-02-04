use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap, VecDeque};

use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::graph_node::{Graph, GraphNode, GraphNodeID};
use crate::config::{CONFIG};
use crate::path::Path;

#[derive(Clone, Debug)]
pub struct SchemaContext {
    #[allow(dead_code)]
    pub id: ID,
    pub lineage: Lineage,
    pub schema_node: Arc<SchemaNode>,
    pub graph_node: Arc<RwLock<GraphNode>>,
}

impl SchemaContext {
    pub fn to_path(
        &self,
        schema_contexts: HashMap<ID, Arc<SchemaContext>>,
    ) -> Result<Path, Errors> {
        log::trace!("In to_path");

        unimplemented!()
    }

    pub fn generate_snippet(&self, meta_context: Arc<RwLock<MetaContext>>) -> String {
        log::trace!("In generate_snippet");

        let mut neighbour_ids = HashSet::new();
        let graph_node = self.graph_node.clone();

        Self::traverse_for_neighbours(
            Arc::clone(&graph_node),
            &mut neighbour_ids
        );

        let lock = read_lock!(meta_context);
        let graph_root = lock.schema_graph_root.clone().unwrap();
        let target_id = read_lock!(graph_node).id.clone();

        let schema_contexts: HashMap<ID, Arc<SchemaContext>> = {
            let lock = read_lock!(meta_context);
            lock.schema_contexts
                .clone()
                .unwrap()
        };

        let snippet = Self::traverse_for_snippet(
            &schema_contexts,
            Arc::clone(&graph_root),
            &|id| neighbour_ids.contains(id),
            &|id| *id == target_id,
        );

        format!("{{ {} }}", snippet)
    }

    pub fn traverse_for_snippet<F, G>(
        schema_contexts: &HashMap<ID, Arc<SchemaContext>>,
        current_node: Graph,
        is_neighbour: &F,
        is_target: &G,
    ) -> String 
    where
        F: Fn(&GraphNodeID) -> bool,
        G: Fn(&GraphNodeID) -> bool,
    {
        let id = {
            let lock = read_lock!(current_node);
            &lock.id.clone()
        };
        let is_current_neighbour = is_neighbour(id);
        let is_current_target = is_target(id);

        let schema_node: Arc<SchemaNode> = {
            let schema_context = schema_contexts
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
                    schema_contexts,
                    Arc::clone(child),
                    is_neighbour,
                    is_target,
                )
            })
            .filter(|item| !item.is_empty())
            .collect();

        let json_schema_key: String = {
            if is_current_target {
                format!(r#"START TARGET SCHEMA KEY >>>{}<<< END TARGET SCHEMA KEY"#, schema_node.name)
            } else {
                schema_node.name.clone()
            }
        };

        if is_current_neighbour || is_current_target {
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

            let schema_neighbour_count = read_lock!(CONFIG).llm.schema_neighbour_count;

            // We want schema snippets to always go back to the root node
            // so we know how to apply json path transformations
            // but we still avoid including distant subgraphs like this
            if visited.len() <= schema_neighbour_count {
                for child in lock.children.iter() {
                    queue.push_back(Arc::clone(child));
                }
            }

            for parent in lock.parents.iter() {
                queue.push_back(Arc::clone(parent));
            }
        }
    }
}
