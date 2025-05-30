use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::graph_node::{Graph, GraphNode};
use crate::context::{Context};

pub struct MetaContext {
    pub contexts: Option<HashMap<ID, Arc<Context>>>,
    pub graph_root: Option<GraphRoot>,
    pub basis_nodes: Option<HashMap<ID, Arc<BasisNode>>>,
    pub basis_networks: Option<HashMap<ID, Arc<BasisNetwork>>>,
    pub basis_graph: Option<BasisGraph>,
    pub profile: Option<Profile>,
}

impl MetaContext {
    pub fn new() -> Self {
        MetaContext {
            contexts: Vec::new(),
            graph_root: None,
            basis_nodes: Vec::new(),
            basis_networks: Vec::new(),
            basis_graph: None,
            profile: None,
        }
    }

    pub fn update_profile(self: &Arc<RwLock<Self>>, profile: Profile) {
        let lock = write_lock!(self);
        lock.profile = Some(profile);
    }

    pub fn update_document_traversal(self: &Arc<RwLock<Self>>, contexts: Vec<Context>, graph_root: GraphRoot) {
        let lock = write_lock!(self);
        lock.contexts = Some(contexts);
        lock.graph_root = Some(graph_root);
    }

    pub fn update_basis_graph(self: &Arc<RwLock<Self>>, graph: BasisGraph) {
        let lock = write_lock!(self);
        lock.basis_graph = Some(graph);
    }

    pub fn update_basis_nodes(self: &Arc<RwLock<Self>>, nodes: Vec<BasisNode>) {
        let lock = write_lock!(self);
        lock.basis_nodes = Some(nodes);
    }

    pub fn update_basis_networks(self: &Arc<RwLock<Self>>, networks: Vec<BasisNetwork>) {
        let lock = write_lock!(self);
        lock.basis_networks = Some(networks);
    }

    pub fn get_original_document(&self) -> String {
        log::trace!("In get_original_document");

        let mut document = String::new();
        let mut visited_lineages: HashSet<Lineage> = HashSet::new();
        let root_node = self.graph_root.unwrap().clone();

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
    let current_context = meta_context.contexts.unwrap();
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
