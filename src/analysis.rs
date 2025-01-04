use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::json_node::{JsonNode};
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document, DocumentType};
use crate::document_format::{DocumentFormat};
use crate::transformation::{Transformation, HashTransformation};
use crate::provider::Provider;
use crate::context::Context;
use crate::document_node::DocumentNode;
use crate::graph_node::{GraphNode, Graph};

pub struct Analysis {
    analysis_mode: AnalysisMode,
    document: Document,
    basis_graph: BasisGraphBuilder,
    data_nodes: HashMap<ID, DataNode>,
    json_nodes: HashMap<ID, JsonNode>,
    value_transformations: Vec<Transformation>,
}

impl Analysis {
    pub fn from_document(
        document: Document,
        options: &Option<Options>
    ) -> Self {
        let default_analysis_mode = AnalysisMode::COMPLEX;

        let analysis_mode = options
            .as_ref()
            .and_then(|opts| opts.analysis_mode.clone())
            .unwrap_or(default_analysis_mode);

        let value_transformations = options
            .as_ref()
            .and_then(|opts| opts.value_transformations.clone())
            .unwrap_or_else(Vec::new);

        Analysis {
            analysis_mode,
            document,
            basis_graph: BasisGraphBuilder::new(),
            data_nodes: HashMap::new(),
            json_nodes: HashMap::new(),
            value_transformations,
        }
    }

    pub fn build_basis_graph(&self) -> Result<BasisGraph, Errors> {
        self.basis_graph.clone().build()
    }
    
    pub async fn transmute(self, target_schema: &str) -> Result<Self, Errors> {
        unimplemented!()
    }

    pub async fn perform_analysis<P: Provider>(
        &mut self,
        provider: &P
    ) -> Result<Self, Errors> {
        log::trace!("In analysis/perform_analysis");

        let profile = self.document.perform_analysis(provider).await?;







        let mut context = Context::new();

        let document_root: DocumentNode = self.document.get_document_node();

        let mut data_nodes: HashMap<ID, DataNode> = HashMap::new();


        fn traverse(
            document_node: DocumentNode,
            parent_lineage: &Lineage,
            context: &mut Context,
            data_nodes: &mut HashMap<ID, DataNode>,
            profile: &Profile,
        ) -> Graph {
            let context_id = context.register(&document_node);

            let data_node = DataNode::new(
                profile.hash_transformation.unwrap(),
                context_id.clone(),
                document_node.get_fields(),
                document_node.get_description(),
                parent_lineage,
            );
            data_nodes.insert(data_node.id.clone(), data_node.clone());




            let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(&data_node)));

            { 
                let children: Vec<Arc<RwLock<GraphNode>>> = document_node
                    .get_children(profile.document_transformations.unwrap())
                    .iter()
                    .map(|child| {
                        traverse(
                            child.clone(),
                            &data_node.lineage,
                            context,
                            data_nodes,
                            profile,
                        )
                    })
                    .collect();

                let mut node_write_lock = graph_node.write().unwrap();
                node_write_lock.children.extend(children);
            }

            graph_node
        }

        let graph = traverse(
            document_root,
            &Lineage::new(),
            &mut context,
            &mut data_nodes,
            &profile,
        );

        self.data_nodes = data_nodes;





        for data_node in self.data_nodes.values() {

            log::debug!("========================================================");

            log::debug!("Data node ID: {}", data_node.id.to_string());
            log::debug!("Data node description: {}", data_node.description.to_string());
            log::debug!("Data node context ID: {}", data_node.context_id.to_string());
            log::debug!("Data node hash: {}", data_node.hash.to_string().unwrap());
            log::debug!("Data node lineage: {}", data_node.lineage.to_string());

            log::debug!("---");
            for (key, value) in &data_node.fields {
                log::debug!("Data node field: {}: {}", key, value);
            }
            log::debug!("---");

        }








        unimplemented!()
    }

    pub fn to_document(self, document_format: &Option<DocumentFormat>) -> Result<Document, Errors> {
        unimplemented!()
    }

    fn to_json(self) -> String {
        unimplemented!()
    }

    fn to_html(self) -> String {
        unimplemented!()
    }

    fn to_xml(self) -> String {
        unimplemented!()
    }

    fn to_text(self) -> String {
        unimplemented!()
    }
}
