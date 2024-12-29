use std::collections::HashMap;

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::json_node::{JsonNode};
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document};
use crate::document_format::{DocumentFormat};
use crate::transformation::{Transformation};
use crate::provider::Provider;

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

        self.document.perform_analysis(provider).await?;
        self.document.apply_transformations()?;




        //let document_root = self.document.get_root_node(&self.context);

        //let data_nodes: HashMap<ID, DataNode> = HashMap::from(
        //    vec![
        //        document_root.0.id.to_string(),
        //        document_root.0.clone()
        //    ]
        //);

        //self.data_nodes = data_nodes;

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



//        fn recurse(
//            document_data: (DataNode, Vec<DocumentNode>),
//            parents: Vec<Rc<GraphNode>>
//        ) {
//            let data_node = document_data.0;
//
//            data_nodes.insert(data_node.id.to_string(), data_node.clone());
//
//            let mut graph_node = GraphNode {
//                id: ID::new(),
//                parents,
//                children: Vec::new(),
//                origin_node_id: document_data.0.id.to_string()
//            };
//
//            let children: document_data.1.iter().map(|child| {
//                recurse(
//                    Document::document_to_data(child, Some(nodes.0)),
//                    Rc::new(graph_node),
//                )
//            });
//
//            graph_node.children.extend(children);
//
//            graph_node
//        }
