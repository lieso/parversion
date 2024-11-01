use tokio::runtime::Runtime;
use std::process;
use std::io::{Read, Write};
use std::fs::File;
use std::sync::{Arc};
use std::collections::{HashSet, HashMap};

mod error;
mod llm;
mod node_data;
mod node_data_structure;
mod utility;
mod xml_node;
mod config;
mod constants;
mod basis_node;
mod graph_node;
mod macros;
mod basis_graph;
mod content;
mod environment;
mod harvest;

pub use graph_node::GraphNodeData;
pub use graph_node::GraphNode;
pub use graph_node::Graph;
pub use basis_node::BasisNode;
pub use basis_graph::BasisGraph;
pub use content::{
    Content,
    ContentMetadata,
    ContentValue,
    ContentValueMetadata,
};
pub use harvest::{Harvest, HarvestFormats, serialize_harvest};

use graph_node::{
    absorb,
    cyclize,
    prune,
    analyze_nodes,
    graph_hash,
};
use basis_graph::{build_basis_graph, analyze_graph};
use xml_node::{XmlNode};
use error::{Errors};
use harvest::{harvest};

pub struct NormalizeResult {
    pub basis_graph: BasisGraph,
    pub harvest: Harvest,
}

pub fn normalize_text(
    text: String,
    input_basis_graph: Option<BasisGraph>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_text");

    if text.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    return Runtime::new().unwrap().block_on(async {
        if utility::is_valid_xml(&text) {
            log::info!("Document is valid XML");

            let result = normalize_xml(&text, input_basis_graph).await?;

            return Ok(result);
        }

        if let Some(xml) = utility::string_to_xml(&text) {
            log::info!("Managed to convert string to XML");

            let result = normalize_xml(&xml, input_basis_graph).await?;

            return Ok(result);
        }

        Err(Errors::UnexpectedDocumentType)
    });
}

pub fn normalize_file(
    file_name: &str,
    input_basis_graph: Option<BasisGraph>
) -> Result<NormalizeResult, Errors> {
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

    normalize_text(document, input_basis_graph)
}

pub async fn normalize_xml(
    xml: &str,
    input_basis_graph: Option<BasisGraph>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(xml);
    log::info!("Done preprocessing XML");

    let input_graph: Graph<XmlNode> = graph_node::build_graph(xml.clone());
    let output_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());

    cyclize(Arc::clone(&input_graph));
    log::info!("Done cyclizing input graph");

    prune(Arc::clone(&input_graph));
    log::info!("Done pruning input graph");

    let input_graph_copy: Graph<XmlNode> = deep_copy(
        Arc::clone(&input_graph),
        vec![GraphNode::from_void()],
        &mut HashSet::new(),
        &mut HashMap::new()
    );

    let basis_graph: BasisGraph = if let Some(previous_basis_graph) = input_basis_graph {
        log::info!("Received a basis graph as input");

        if !previous_basis_graph.contains_subgraph(Arc::clone(&input_graph)) {
            log::info!("Input graph is not a subgraph of basis graph");
            absorb(Arc::clone(&previous_basis_graph.root), Arc::clone(&input_graph));
        }

        previous_basis_graph
    } else {
        log::info!("Did not receive a basis graph as input");
        build_basis_graph(Arc::clone(&input_graph))
    }

    log::info!("Performing network analysis...");
    analyze_graph(basis_graph, Arc::clone(&input_graph_copy));

    log::info!("Performing node analysis...");
    analyze_nodes(basis_graph, Arc::clone(&output_tree));

    log::info!("Harvesting output tree..");
    let harvest = harvest(Arc::clone(&output_tree), basis_graph.clone());

    Ok(NormalizeResult {
        basis_graph: basis_graph,
        harvest: harvest,
    })
}
