use std::sync::{Arc};
use std::collections::{HashSet, HashMap};

use crate::prelude::*;
use crate::graph_node::{Graph};
use crate::context::{Context};
use crate::basis_graph::BasisGraph;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::profile::Profile;
use crate::transformation::SchemaTransformation;
use crate::document::{Document};
use crate::schema::Schema;
use crate::schema_context::SchemaContext;

pub struct MetaContext {
    pub document_versions: HashMap<DocumentVersion, Arc<Document>>,
    pub contexts: Option<HashMap<ID, Arc<Context>>>,
    pub graph_root: Option<Graph>,
    pub basis_nodes: Option<HashMap<ID, Arc<BasisNode>>>,
    pub basis_networks: Option<HashMap<ID, Arc<BasisNetwork>>>,
    pub basis_graph: Option<Arc<BasisGraph>>,
    pub profile: Option<Arc<Profile>>,
    pub schema_contexts: Option<HashMap<ID, Arc<SchemaContext>>>,
    pub schema_graph_root: Option<Graph>,
    pub schema_transformations: Option<HashMap<Lineage, Arc<SchemaTransformation>>>,
    pub translation_schema: Option<Arc<Schema>>,
    pub translation_schema_contexts: Option<HashMap<ID, Arc<SchemaContext>>>,
    pub translation_schema_graph_root: Option<Graph>,
}

impl MetaContext {
    pub fn new() -> Self {
        MetaContext {
            document_versions: HashMap::new(),
            contexts: None,
            graph_root: None,
            basis_nodes: None,
            basis_networks: None,
            basis_graph: None,
            profile: None,
            schema_contexts: None,
            schema_graph_root: None,
            schema_transformations: None,
            translation_schema: None,
            translation_schema_contexts: None,
            translation_schema_graph_root: None,
        }
    }

    pub fn add_document_version(
        &mut self,
        document_version: DocumentVersion,
        document: Document
    ) {
        self.document_versions.insert(document_version, Arc::new(document));
    }

    pub fn get_document(
        &self,
        version: DocumentVersion
    ) -> Option<Arc<Document>> {
        self.document_versions.get(&version).cloned()
    }

    pub fn update_translation_schema(
        &mut self,
        schema: Schema
    ) {
        self.translation_schema = Some(Arc::new(schema));
    }

    pub fn update_schema_context(
        &mut self,
        contexts: HashMap<ID, Arc<SchemaContext>>,
        graph_root: Graph
    ) {
        self.schema_contexts = Some(contexts);
        self.schema_graph_root = Some(graph_root);
    }

    pub fn update_translation_schema_context(
        &mut self,
        contexts: HashMap<ID, Arc<SchemaContext>>,
        graph_root: Graph
    ) {
        self.translation_schema_contexts = Some(contexts);
        self.translation_schema_graph_root = Some(graph_root);
    }

    pub fn get_basis_network_by_subgraph_hash(
        &self,
        subgraph_hash: &String
    ) -> Result<Option<Arc<BasisNetwork>>, Errors> {
        log::trace!("In get_basis_network_by_subgraph_hash");
        
        let basis_networks = self.basis_networks.as_ref().unwrap();

        for basis_network in basis_networks.values() {
            if basis_network.subgraph_hash == *subgraph_hash {
                return Ok(Some(Arc::clone(&basis_network)));
            }
        }

        Ok(None)
    }

    pub fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<Arc<BasisNode>>, Errors> {
        log::trace!("In get_basis_node_by_lineage");

        let basis_nodes = self.basis_nodes.as_ref().unwrap();

        for basis_node in basis_nodes.values() {
            if basis_node.lineage == *lineage {
                return Ok(Some(Arc::clone(&basis_node)));
            }
        }

        Ok(None)
    }

    pub fn update_schema_transformations(
        &mut self,
        schema_transformations: HashMap<Lineage, Arc<SchemaTransformation>>
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

