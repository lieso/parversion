use std::sync::{Arc};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::graph_node::{Graph};
use crate::context::{Context};
use crate::basis_graph::BasisGraph;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::profile::Profile;
use crate::transformation::SchemaTransformation;

pub struct MetaContext {
    pub contexts: Option<HashMap<ID, Arc<Context>>>,
    pub graph_root: Option<Graph>,
    pub basis_nodes: Option<HashMap<ID, Arc<BasisNode>>>,
    pub basis_networks: Option<HashMap<ID, Arc<BasisNetwork>>>,
    pub basis_graph: Option<Arc<BasisGraph>>,
    pub profile: Option<Arc<Profile>>,
    pub schema_transformations: Option<HashMap<ID, Arc<SchemaTransformation>>>,
}

impl MetaContext {
    pub fn new() -> Self {
        MetaContext {
            contexts: None,
            graph_root: None,
            basis_nodes: None,
            basis_networks: None,
            basis_graph: None,
            profile: None,
            schema_transformations: None,
        }
    }

    pub fn update_schema_transformations(
        &mut self,
        schema_transformations: HashMap<ID, Arc<SchemaTransformation>>
    ) {
        self.schema_transformations = Some(schema_transformations);
    }

    pub fn update_profile(&mut self, profile: Arc<Profile>) {
        self.profile = Some(profile);
    }

    pub fn update_data_structures(&mut self, contexts: HashMap<ID, Arc<Context>>, graph_root: Graph) {
        self.contexts = Some(contexts);
        self.graph_root = Some(graph_root);
    }

    pub fn update_basis_graph(&mut self, graph: Arc<BasisGraph>) {
        self.basis_graph = Some(graph);
    }

    pub fn update_basis_nodes(&mut self, nodes: HashMap<ID, Arc<BasisNode>>) {
        self.basis_nodes = Some(nodes);
    }

    pub fn update_basis_networks(&mut self, networks: HashMap<ID, Arc<BasisNetwork>>) {
        self.basis_networks = Some(networks);
    }

    pub fn get_original_document(&self) -> String {
        log::trace!("In get_original_document");

        let mut document = String::new();
        let mut visited_lineages: HashSet<Lineage> = HashSet::new();
        let root_node = self.graph_root.clone().unwrap();

        traverse_for_condensed_document(
            self,
            Arc::clone(&root_node),
            &mut visited_lineages,
            &mut document,
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
    let current_context = meta_context.contexts.clone().unwrap();
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
