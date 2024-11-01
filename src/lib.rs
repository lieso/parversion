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
    interpret,
    graph_hash,
    deep_copy,
    to_xml_string
};
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

    read_lock!(input_graph).debug_visualize("input_graph");

    cyclize(Arc::clone(&input_graph));
    log::info!("Done cyclizing input graph");

    prune(Arc::clone(&input_graph));
    log::info!("Done pruning input graph");

    read_lock!(input_graph).debug_statistics("pruned_input_graph");
    read_lock!(input_graph).debug_visualize("pruned_input_graph");

    let subgraph_hash = graph_hash(Arc::clone(&input_graph));
    log::debug!("subgraph_hash: {}", subgraph_hash);

    let pruned_input: String = to_xml_string(Arc::clone(&input_graph));

    if environment::is_local() {
        let mut file = File::create("./debug/pruned_input.xml").expect("Could not create file");
        file.write_all(pruned_input.as_bytes()).expect("Could not write to file");
    }

    let basis_graph = if let Some(previous_basis_graph) = input_basis_graph {
        log::info!("Received a basis graph as input");

        let basis_root: Graph<BasisNode> = previous_basis_graph.root;
        let mut subgraph_hashes = previous_basis_graph.subgraph_hashes;

        log::info!("previous subgraph hashes: {:?}", subgraph_hashes);

        if !subgraph_hashes.contains(&subgraph_hash) {
            log::info!("Input graph is not a subgraph of basis graph");

            absorb(Arc::clone(&basis_root), Arc::clone(&input_graph));

            subgraph_hashes.push(subgraph_hash);

            log::info!("Interpreting basis graph...");
            interpret(Arc::clone(&basis_root), Arc::clone(&output_tree)).await;
            log::info!("Done interpreting basis graph.");
        }

        BasisGraph {
            root: basis_root,
            subgraph_hashes: subgraph_hashes,
        }
    } else {
        log::info!("Did not receive a basis graph as input");

        let copy: Graph<BasisNode> = deep_copy(
            Arc::clone(&input_graph),
            vec![GraphNode::from_void()],
            &mut HashSet::new(),
            &mut HashMap::new()
        );
        let new_root: Graph<BasisNode> = GraphNode::from_void();
        {
            write_lock!(new_root).children.push(Arc::clone(&copy));
        }
        read_lock!(new_root).debug_visualize("new_root");

        log::info!("Interpreting basis graph...");
        interpret(Arc::clone(&new_root), Arc::clone(&output_tree)).await;
        log::info!("Done interpreting basis graph.");

        BasisGraph {
            root: new_root,
            subgraph_hashes: vec![subgraph_hash]
        }
    };

    log::info!("Harvesting output tree..");
    let harvest = harvest(Arc::clone(&output_tree), basis_graph.clone());
    log::info!("Done harvesting output tree.");

    Ok(NormalizeResult {
        basis_graph: basis_graph,
        harvest: harvest,
    })
}
