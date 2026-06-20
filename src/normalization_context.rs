use std::collections::HashMap;
use std::sync::Arc;

use crate::basis_field::BasisField;
use crate::basis_group::BasisGroup;
use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::basis_graph::BasisGraph;
use crate::context::Context;
use crate::document::Document;
use crate::graph_node::Graph;
use crate::meta_context::MetaContext;
use crate::prelude::*;
use crate::normal_context::NormalContext;

pub struct NormalizationContext {
    pub document_versions: HashMap<DocumentVersion, Arc<Document>>,
    pub meta_context: Option<Arc<MetaContext>>,
    pub basis_fields: Option<HashMap<ID, Arc<BasisField>>>,
    pub basis_groups: Option<HashMap<ID, Arc<BasisGroup>>>,
    pub basis_nodes: Option<HashMap<ID, Arc<BasisNode>>>,
    pub basis_networks: Option<HashMap<ID, Arc<BasisNetwork>>>,
    pub basis_graph: Option<BasisGraph>,
    pub classification: Option<Arc<Classification>>,
    pub normal_contexts: Option<HashMap<ID, Arc<NormalContext>>>,
    pub normal_graph_root: Option<Graph>,
    pub context_groups: Option<HashMap<ID, Vec<Arc<Context>>>>,
    pub context_to_group: Option<HashMap<ID, Arc<BasisGroup>>>,
}

impl NormalizationContext {
    pub fn new() -> Self {
        NormalizationContext {
            document_versions: HashMap::new(),
            meta_context: None,
            basis_fields: None,
            basis_groups: None,
            basis_nodes: None,
            basis_networks: None,
            basis_graph: None,
            classification: None,
            normal_contexts: None,
            normal_graph_root: None,
            context_groups: None,
            context_to_group: None,
        }
    }

    pub fn add_document_version(&mut self, document_version: DocumentVersion, document: Document) {
        self.document_versions
            .insert(document_version, Arc::new(document));
    }

    pub fn get_document(&self, version: DocumentVersion) -> Option<Arc<Document>> {
        self.document_versions.get(&version).cloned()
    }

    // TODO: LINEAGE!
    pub fn get_basis_network_by_lineage_and_subgraph_hash(
        &self,
        subgraph_hash: &Hash,
    ) -> Result<Option<Arc<BasisNetwork>>, Errors> {
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
        lineage: &Lineage,
    ) -> Result<Option<Arc<BasisNode>>, Errors> {
        let basis_nodes = self.basis_nodes.as_ref().unwrap();

        for basis_node in basis_nodes.values() {
            if basis_node.lineage == *lineage {
                return Ok(Some(Arc::clone(&basis_node)));
            }
        }

        Ok(None)
    }

    pub fn update_meta_context(&mut self, meta_context: MetaContext) {
        self.meta_context = Some(Arc::new(meta_context));
    }
    
    pub fn update_normalized_graph(
        &mut self,
        contexts: HashMap<ID, Arc<NormalContext>>,
        graph_root: Graph,
    ) {
        self.normal_contexts = Some(contexts);
        self.normal_graph_root = Some(graph_root);
    }

    pub fn update_classification(&mut self, classification: Arc<Classification>) {
        self.classification = Some(classification);
    }

    pub fn get_classification(&self) -> Option<Arc<Classification>> {
        self.classification.as_ref().map(Arc::clone)
    }

    pub fn update_basis_groups(&mut self, groups: HashMap<ID, Arc<BasisGroup>>) {
        self.basis_groups = Some(groups);
    }

    pub fn update_basis_fields(&mut self, fields: HashMap<ID, Arc<BasisField>>) {
        self.basis_fields = Some(fields);
    }

    pub fn update_basis_nodes(&mut self, nodes: HashMap<ID, Arc<BasisNode>>) {
        self.basis_nodes = Some(nodes);
    }

    pub fn update_basis_networks(&mut self, networks: HashMap<ID, Arc<BasisNetwork>>) {
        self.basis_networks = Some(networks);
    }

    pub fn update_basis_graph(&mut self, basis_graph: BasisGraph) {
        self.basis_graph = Some(basis_graph);
    }

    pub fn update_context_groups(
        &mut self,
        context_groups: HashMap<ID, Vec<Arc<Context>>>,
        context_to_group: HashMap<ID, Arc<BasisGroup>>
    ) {
        self.context_groups = Some(context_groups);
        self.context_to_group = Some(context_to_group);
    }
}
