use crate::prelude::*;
use crate::basis_node::{BasisNode};
use crate::data_node::{DataNode};
use crate::json_node::{JsonNode};
use crate::basis_graph::{BasisGraph};
use crate::document::{Document};

pub struct Analysis {
    pub options: Option<Options>,
    pub document: Document,
    pub document_context: Context,
    pub document_transformations: Vec<DocumentTransformation>,
    pub basis_nodes: HashMap<ID, BasisNode>,
    pub basis_name: Option<String>,
    pub basis_description: Option<String>,
    pub basis_networks: HashMap<ID, BasisNetwork>,
    pub data_nodes: HashMap<ID, DataNode>,
    pub json_nodes: HashMap<ID, JsonNode>,
    pub json_schema: Option<String>,
    pub value_transformations: Vec<ValueTransformation>,
}

impl Analysis {
    pub fn from_document(
        document: Document,
        options: Option<Options>
    ) -> Self {
        let context: Context = Context::new();

        Analysis {
            options,
            document,
            document_context: context,
            document_transformations: Vec::new(),
            basis_nodes: HashMap::new(),
            basis_name: None,
            basis_description: None,
            basis_networks: HashMap::new(),
            data_nodes: HashMap::new(),
            json_nodes: HashMap::new(),
            json_schema: None,
            value_transformations: Vec::new(),
        }
    }

    pub fn to_document(self, document_type: DocumentType) -> Document {
        unimplemented!()
    }

    pub fn with_basis(self, basis_graph: &BasisGraph) -> Self {
        self.basis_name = Some(basis_graph.name.clone());
        self.basis_description = Some(basis_graph.description.clone());
        self.basis_nodes = Some(basis_graph.nodes.clone());
        self.basis_networks = Some(basis_graph.networks.clone());

        self
    }

    pub fn with_value_transformations(self, value_transformations: Vec<ValueTransformation>) -> Self {
        self.value_transformations = value_transformations;

        self
    }

    pub fn get_basis_graph(self) -> BasisGraph {
        BasisGraph {
            id: ID::new(),
            name: self.basis_name.clone().unwrap(),
            description: self.basis_description.clone().unwrap(),
            json_schema: self.json_schema.clone().unwrap(),
            nodes: self.basis_nodes.clone().unwrap(),
            networks: self.basis_networks.clone().unwrap(),
        }
    }
    
    pub async fn transmute(self, target_schema: &str) -> Result<Self, Errors> {
        unimplemented!()
    }

    pub async fn perform_analysis(self) -> Result<Self, Errors> {




        let document_transformations = self.document.perform_analysis();

        self.document.apply_transformations(document_transformations);







        let document_root = self.document.get_root_node(&self.context);

        let data_nodes: HashMap<ID, DataNode> = HashMap::from(
            vec![
                document_root.0.id.to_string(),
                document_root.0.clone()
            ]
        );

        self.data_nodes = data_nodes;

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
