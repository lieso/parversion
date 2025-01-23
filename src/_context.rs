use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::id::{ID};
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::macros::*;

type ContextID = ID;
type GraphID = ID;

pub struct Context {
}

impl Context {
    pub fn get_snippet(
        &self,
        context_id: &ContextID,
        document_nodes: &HashMap<ContextID, Arc<RwLock<DocumentNode>>>, 
        graph_nodes: &HashMap<ContextID, Arc<RwLock<GraphNode>>,
        root_context_id: &ID,
    ) -> String {
        log::trace!("In get_snippet");


        let document_node = document_nodes.get(context_id).unwrap();


        let graph_node = graph_nodes.get(context_id).clone().unwrap();


        let mut document_node_ids: HashSet<ID> = HashSet::new();
        self.traverse_neighbours(
            Arc::clone(graph_node),
            &mut document_node_ids,
            document_nodes
        );

        log::debug!("document_node_ids: {:?}", document_node_ids);


        let mut snippet = String::new();

        let root_document_node = document_nodes.get(&root_context_id).unwrap();

        //self.traverse_document(
        //    &mut snippet,
        //    &root_document_node,
        //    &document_node_ids,
        //    &read_lock!(document_node).id
        //);

        log::debug!("-----------------------------------------------------------------------------------------------------");
        log::debug!("snippet: {}", snippet);


        unimplemented!()
    }

    //fn traverse_document(
    //    &self,
    //    snippet: &mut String,
    //    document_node: Arc<RwLock<DocumentNode>>,
    //    document_node_ids: &HashSet<ID>,
    //    target_id: &ID
    //) {
    //    let document_node = read_lock!(document_node);

    //    let (mut a, b) = document_node.to_string_components();

    //    if document_node.id == *target_id {
    //        a = Context::mark_text(&a);
    //    }

    //    let should_render = document_node_ids.contains(&document_node.id);

    //    if let Some(closing_tag) = &b {
    //        if should_render {
    //            snippet.push_str(&a);
    //        }

    //        for child in document_node.get_children(None) {
    //            self.traverse_document(
    //                snippet,
    //                &child,
    //                document_node_ids,
    //                target_id
    //            );
    //        }

    //        if should_render {
    //            snippet.push_str(closing_tag);
    //        }
    //    } else if should_render {
    //        snippet.push_str(&a);
    //    }
    //}

    fn traverse_neighbours(
        &self,
        graph_node: Graph,
        visited: &mut HashSet<ID>,
        document_nodes: &HashMap<ContextID, Arc<RwLock<DocumentNode>>>, 
        graph_context: &HashMap<GraphID, ContextID>,
    ) {
        let mut stack = VecDeque::new();
        stack.push_back(Arc::clone(&graph_node));

        while let Some(node) = stack.pop_back() {
            let lock = read_lock!(node);
            let context_id = graph_context.get(&lock.id).unwrap();
            let document_node = document_nodes.get(context_id).unwrap();

            if visited.contains(&read_lock!(document_node).id) {
                continue;
            }

            visited.insert(read_lock!(document_node).id.clone());

            if visited.len() > 20 {
                return;
            }

            for child in lock.children.iter() {
                let child_id = &read_lock!(child).id;
                let context_id = graph_context.get(child_id).unwrap();
                let document_node = document_nodes.get(&context_id).unwrap();

                if !visited.contains(&read_lock!(document_node).id) {
                    stack.push_back(Arc::clone(child));
                }
            }

            if let Some(parent) = lock.parents.first() {
                let parent_id = &read_lock!(parent).id;
                let context_id = graph_context.get(parent_id).unwrap();
                let document_node = document_nodes.get(&context_id).unwrap();

                if !visited.contains(&read_lock!(document_node).id) {
                    stack.push_back(Arc::clone(parent));
                }
            }
        }
    }

    fn mark_text(text: &str) -> String {
        let marker_prefix = "<!-- Target node: Start -->";
        let marker_suffix = "<!-- Target node: End -->";

        format!("{}{}{}", marker_prefix, text, marker_suffix)
    }
}
