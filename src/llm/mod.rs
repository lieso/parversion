use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};
use crate::graph_node::{GraphNode, GraphNodeID};

mod openai;


pub struct LLM {}

impl LLM {
    pub async fn get_field_transformation(
        meta_context: Arc<MetaContext>,
        context_group: Vec<Arc<Context>>,
    ) -> Result<Option<FieldTransformation>, Errors> {
        log::trace!("In get_field_transformation");

        let snippets: Vec<String> = context_group.iter().map(|context| {
            Self::generate_snippet(
                Arc::clone(&meta_context),
                Arc::clone(&context)
            )
        }).collect();


        unimplemented!()
        //openai::OpenAI::get_field_transformation(field, value, snippet).await
    }

    fn generate_snippet(meta_context: Arc<MetaContext>, context: Arc<Context>) -> String {
        log::trace!("In generate_snippet");

        let mut neighbour_ids = HashSet::new();
        
        Self::traverse_for_neighbours(
            Arc::clone(&meta_context.graph_root),
            &mut neighbour_ids
        );

        let mut snippet = String::new();

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
}
