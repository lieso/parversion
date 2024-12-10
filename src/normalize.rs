use tokio::runtime::Runtime;
use std::process;
use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use std::collections::{HashSet, HashMap};
use serde_json::{Value};

use crate::graph_node::GraphNode;
use crate::graph_node::Graph;
use crate::basis_graph::BasisGraph;
use crate::harvest::{Harvest, HarvestFormats, serialize_harvest};
use crate::graph_node;
use crate::basis_graph::{
    build_basis_graph,
    analyze_graph,
};
use crate::xml_node::{XmlNode};
use crate::error::{Errors};
use crate::harvest::{harvest};
use crate::utility;
use crate::macros::*;
use crate::json_schema::{
    content_to_json_schema,
    get_schema_mapping,
    apply_schema_mapping
};

pub struct Normalization {
    pub output_basis_graph: BasisGraph,
    pub normalized: Value,
    pub related: Value,
}

pub fn normalize_text(
    origin: Option<String>,
    date: Option<String>,
    text: String,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_text");

    return Runtime::new().unwrap().block_on(async {

        let document = Document::from_string(text, origin, date)?;

        let transformations: Vec<DocumentTransformation> = Vec::new();

        document.apply_transformations(transformations);


    });
}

pub fn normalize_file(
    url: Option<String>,
    file_name: String,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file_name: {}", file_name);

    let mut document = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut document).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    normalize_text(url, document)
}

pub async fn normalize_document(
    document: Document,
    basis_locations: Vec<String>,
    value_transformations: Vec<Transformation>
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document");

    let input_graph: Graph<XmlNode> = build_unique_graph(xml.clone());


    let xml_string: String = to_xml_string(Arc::clone(&input_graph));




    let llm_classification = get_graph_type_id(snippets, xml_string).await;





    normalize_document_with_mutable_basis(url, xml, basis_graph)
}

pub async fn normalize_document_without_basis(
    document: Document,
    value_transformations: Vec<Transformation>
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document_without_basis");



    let mut (root_node, root_node_children) = docment.get_root_node();

    let data_nodes: HashMap<ID, DataNode> = HashMap::from(
        vec![current_node.id.to_string(), current_node.clone()]
    );


    fn recurse(
        document_data: (DataNode, Vec<DocumentNode>),
        parents: Vec<Rc<GraphNode>>
    ) {

        let mut graph_node = GraphNode {
            id: ID::new(),
            parents,
            children: Vec::new(),
            origin_node_id: document_data.0.id.to_string()
        };

        let children: document_data.1.iter().map(|child| {
            recurse(
                Document::document_to_data(child, Some(nodes.0)),
                Rc::new(graph_node),
            )
        });

        graph_node.children.extend(children);

    }

    fn recurse(
        document_node: T,
    ) {


        let (data_node, children) = Document::get_data_node(child);

        data_nodes.insert(data_node.id, data_node.clone());

        let graph_node = GraphNode {
            id: ID::new(),
            parents: vec![current_graph_node.clone()],
            children: Vec::new(),
            origin_node_id: data_node.id.to_string(),
        };

        graph_node.children = recurse(
            
        );

    }

    let graph_node_children = recurse(

    )

    let graph_node: GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        children: graph_node_children,
        origin_node_id: current_node.id.to_string()
    };







    let mut basis_graph = DefaultBasisGraph::default();

    let input_graph: Graph<XmlNode> = build_unique_graph(xml.clone());
    let xml_string: String = to_xml_string(Arc::clone(&input_graph));
    let llm_classification = classify_graph(snippets, xml_string).await;

}

pub async fn normalize_document_with_basis_mutable(
    document: Document,
    value_transformations: Vec<Transformation>
    basis_graph: BasisGraph
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document_with_basis_mutable");

    let input_graph: Graph<DataNode> = build_unique_graph(&document);

    basis_graph.perform_node_analysis(Arc::clone(&input_graph)).await;

    let json_tree: Graph<JsonNode>;

    basis_graph.apply_node_transformations(
        Arc::clone(&output_tree),
        Arc::clone(&json_tree)
    );
    basis_graph.perform_network_analysis(Arc::clone(&json_tree)).await;


    let output_tree: Graph<XmlNode> = build_tree(&document);


    if let Some(json_schema) = basis_graph.json_schema {
        basis_graph.transformations = get_schema_transformations(json_schema, source_schema).await;
    } else {
        basis_graph.json_schema = source_schema;
    }

    let normalized = basis_graph.apply_schema_transformations(source_data);

    Normalization {
        output_basis_graph: basis_graph,
        normalized: normalized,
    }
}

pub async fn normalize_document_with_basis_immutable(
    url: Option<String>,
    xml: &str,
    basis_graph: BasisGraph,
    value_transformations: Vec<Transformation>
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document_with_basis_immutable");

    let output_tree: Graph<XmlNode> = build_tree(xml.clone());

    let json_tree: Graph<JsonNode>;

    basis_graph.apply_node_transformations(output_tree, json_tree);
    basis_graph.apply_network_transformations(json_tree);
    basis_graph.apply_value_transformations(json_tree, value_transformations);
    basis_graph.apply_schema_transformations(json_tree);

    let data = basis_graph.collect_nodes(json_graph);

    Normalization {
        output_basis_graph: basis_graph,
        normalized: data
    }
}
