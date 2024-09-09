use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::sync::{Arc};

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
mod traversal;

use basis_node::{
    BasisNode
};
use graph_node::{
    GraphNode,
    Graph,
    absorb,
    cyclize,
    prune,
    interpret,
};
use xml_node::{XmlNode};
use error::{Errors};
use traversal::{Traversal, Harvest};

#[derive(Debug)]
pub enum HarvestFormats {
    JSON,
    //XML,
    //CSV,
    //HTML
}

pub struct NormalizeResult {
    pub basis_graph: Graph<BasisNode>,
    pub harvest: Harvest,
}

pub fn normalize(
    text: String,
    input_basis_graph: Option<Graph<BasisNode>>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize");

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

pub fn normalize_file(file_name: &str) -> Result<NormalizeResult, Errors> {
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

    normalize(document, None)
}

pub async fn normalize_xml(
    xml: &str,
    input_basis_graph: Option<Graph<BasisNode>>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(xml);
    log::info!("Done preprocessing XML");

    let input_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());
    let output_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());

    let basis_graph: Graph<BasisNode> = match input_basis_graph {
        Some(graph) => graph,
        None => GraphNode::from_void(),
    };

    absorb(Arc::clone(&basis_graph), Arc::clone(&input_tree));
    log::info!("Done absorbing input tree into basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_absorbed");

    cyclize(Arc::clone(&basis_graph));
    log::info!("Done cyclizing basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_cyclized");

    prune(Arc::clone(&basis_graph));
    log::info!("Done pruning basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_pruned");
    read_lock!(basis_graph).debug_statistics("basis_graph_pruned");

    log::info!("Interpreting basis graph...");
    interpret(Arc::clone(&basis_graph), Arc::clone(&output_tree)).await;
    log::info!("Done interpreting basis graph.");
    read_lock!(basis_graph).debug_visualize("basis_graph_interpreted");

    let harvest = Traversal::from_tree(Arc::clone(&output_tree))
        .with_basis(Arc::clone(&basis_graph))
        .harvest()?;

    Ok(NormalizeResult {
        basis_graph: basis_graph,
        harvest: harvest,
    })
}

pub fn serialize(harvest: Harvest, format: HarvestFormats) -> Result<String, Errors> {
    match format {
        HarvestFormats::JSON => {
            log::info!("Serializing harvest as JSON");

            let serialized = serde_json::to_string(&harvest).expect("Could not serialize output to JSON");

            Ok(serialized)
        },
    }
}
