use tokio::runtime::Runtime;
use std::process;
use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use std::collections::{HashSet, HashMap};

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
use crate::json_schema::{content_to_json_schema, get_schema_mapping};

pub struct NormalizeResult {
    pub output_basis_graph: BasisGraph,
    pub harvest: Harvest,
}

pub fn normalize_text_js(
    url: Option<String>,
    text: String,
    input_basis_graph: Option<String>,
    other_basis_graphs: Vec<String>,
) -> String {
    log::trace!("In normalize_text_js");

    fn process_basis_graph(graph: &str) -> BasisGraph {
        serde_json::from_str(graph).unwrap()
    }

    let input_basis_graph: Option<Box<BasisGraph>> = input_basis_graph.map(|value| {
        Box::new(process_basis_graph(&value))
    });

    let other_basis_graphs: Vec<BasisGraph> = other_basis_graphs
        .into_iter()
        .map(|item| process_basis_graph(&item))
        .collect();

    let result = normalize_text(url, text, input_basis_graph, other_basis_graphs).unwrap();

    serialize_harvest(result.harvest, HarvestFormats::JSON).expect("Could not serialize results")
}

pub fn normalize_text(
    url: Option<String>,
    text: String,
    input_basis_graph: Option<Box<BasisGraph>>,
    other_basis_graphs: Vec<BasisGraph>,
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_text");

    if text.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    return Runtime::new().unwrap().block_on(async {
        if utility::is_valid_xml(&text) {
            log::info!("Document is valid XML");

            let result = normalize_xml(
                url,
                &text,
                input_basis_graph,
                other_basis_graphs
            ).await?;

            return Ok(result);
        }

        let xml = utility::string_to_xml(&text).expect("Could not convert string to xml");

        let result = normalize_xml(
            url,
            &xml,
            input_basis_graph,
            other_basis_graphs
        ).await?;

        Ok(result)
    });
}

pub fn normalize_file(
    url: Option<String>,
    file_name: String,
    input_basis_graph: Option<Box<BasisGraph>>,
    other_basis_graphs: Vec<BasisGraph>,
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

    normalize_text(url, document, input_basis_graph, other_basis_graphs)
}

pub async fn normalize_xml(
    url: Option<String>,
    xml: &str,
    input_basis_graph: Option<Box<BasisGraph>>,
    other_basis_graphs: Vec<BasisGraph>,
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(url.as_deref(), xml);
    log::info!("Done preprocessing XML");

    let input_graph: Graph<XmlNode> = graph_node::build_graph(xml.clone());
    let output_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());

    graph_node::cyclize(Arc::clone(&input_graph));
    log::info!("Done cyclizing input graph");

    graph_node::prune(Arc::clone(&input_graph));
    log::info!("Done pruning input graph");

    let input_graph_copy: Graph<XmlNode> = graph_node::deep_copy_single(
        Arc::clone(&input_graph),
        vec![GraphNode::from_void()],
        &mut HashSet::new(),
        &mut HashMap::new()
    );

    let mut basis_graph: BasisGraph = if let Some(previous_basis_graph) = input_basis_graph {
        log::info!("Received a basis graph as input");

        if !previous_basis_graph.contains_subgraph(Arc::clone(&input_graph)) {
            log::info!("Input graph is not a subgraph of basis graph");

            graph_node::absorb(
                Arc::clone(&previous_basis_graph.root),
                Arc::clone(&input_graph)
            );
        } else {
            log::info!("Input graph is a subgraph of basis graph");

            log::info!("Harvesting output tree..");
            let basis_graphs = other_basis_graphs
                .into_iter()
                .chain(std::iter::once(*previous_basis_graph.clone()))
                .collect();
            let harvest = harvest(
                Arc::clone(&output_tree),
                basis_graphs,
            );

            return Ok(NormalizeResult {
                output_basis_graph: *previous_basis_graph,
                harvest: harvest,
            });
        }

        *previous_basis_graph
    } else {
        log::info!("Did not receive a basis graph as input");
        build_basis_graph(Arc::clone(&input_graph))
    };

    log::info!("Performing network analysis...");
    analyze_graph(&mut basis_graph, Arc::clone(&input_graph_copy)).await;


    if basis_graph.subgraphs.len() > 1 {
        panic!("Don't know how to handle multiple subgraphs");
    }



    log::info!("Performing node analysis...");
    for subgraph in basis_graph.subgraphs.values_mut().filter(|s| !s.analyzed) {
        log::info!("Analyzing nodes in subgraph with id: {}", subgraph.id);

        graph_node::analyze_nodes(
            Arc::clone(&basis_graph.root),
            Arc::clone(&output_tree),
            &*subgraph,
            &other_basis_graphs
        ).await;
        
        subgraph.analyzed = true;
    }

    log::info!("Harvesting output tree..");
    let basis_graphs: Vec<BasisGraph> = other_basis_graphs
        .into_iter()
        .chain(std::iter::once(basis_graph.clone()))
        .collect();

    let harvest_result = harvest(
        Arc::clone(&output_tree),
        basis_graphs,
    );




    let original_schema = content_to_json_schema(harvest_result.content.clone());
    log::debug!("original_schema: {}", original_schema);

    let page_type = basis_graph.subgraphs.values().next().unwrap().page_type.clone();


    if let Some(known_schema) = page_type.json_schema {
        log::debug!("Content is of a known category: {}", page_type.name);

        let schema_mapping = get_schema_mapping(known_schema, original_schema).await;
    }




    

    Ok(NormalizeResult {
        output_basis_graph: basis_graph,
        harvest: harvest_result,
    })
}
