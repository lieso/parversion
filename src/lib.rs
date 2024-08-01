use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::sync::{Arc, Mutex, RwLock};

mod error;
mod llm;
mod node;
mod node_data;
mod node_data_structure;
mod traversal;
mod utility;
mod xml;
mod config;
mod constants;
mod basis_node;
mod graph;

use node::{
    Node,
    build_tree,
    prune,
    get_tree_metadata,
    linearize,
    interpret,
};
use basis_node::{
    BasisNode
};
use graph::{
    MutexGraph,
    MutexGraphNode,
    RwLockGraph,
    absorb,
};
use xml::{Xml};
use error::{Errors};
use traversal::{Traversal};

pub fn normalize(text: String) -> Result<String, Errors> {
    log::trace!("In normalize");

    if text.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    return Runtime::new().unwrap().block_on(async {
        if utility::is_valid_xml(&text) {
            log::info!("Document is valid XML");

            let result = normalize_xml(&text).await?;

            return Ok(result);
        }

        if let Some(xml) = utility::string_to_xml(&text) {
            log::info!("Managed to convert string to XML");

            let result = normalize_xml(&xml).await?;

            return Ok(result);
        }

        Err(Errors::UnexpectedDocumentType)
    });
}

pub fn normalize_file(file_name: &str) -> Result<String, Errors> {
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

    normalize(document)
}

pub async fn normalize_xml(xml: &str) -> Result<String, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(xml);
    log::info!("Done preprocessing XML");

    let input_tree: MutexGraph<Xml> = graph::build_mutex_graph(xml.clone());
    let output_tree: RwLockGraph<Xml> = graph::build_rwlock_graph(xml.clone());

    std::mem::drop(xml);

    let basis_graph: MutexGraph<BasisNode> = MutexGraphNode::from_void();

    absorb(Arc::clone(&basis_graph), Arc::clone(&input_tree));
    log::info!("Done absorbing input tree into basis graph");

    graph::mutex_bft(Arc::clone(&basis_graph), &mut |node: MutexGraph<BasisNode>| {
        let guard = node.lock().unwrap();
        log::debug!("hash: {}", guard.hash);
    });


    graph::rwlock_bft(Arc::clone(&output_tree), &mut |node: RwLockGraph<Xml>| {
        let guard = node.read().unwrap();
        log::debug!("hash: {}", guard.hash);
    });




    unimplemented!()
}

//pub async fn normalize_xml(xml_string: &str) -> Result<String, Errors> {
//    log::trace!("In normalize_xml");
//
//    let xml = utility::preprocess_xml(xml_string);
//    log::info!("Done preprocessing XML");
//
//    let input_tree: Rc<Node> = build_tree(xml.clone());
//    let output_tree: Rc<Node> = deep_copy(&input_tree);
//
//    let basis_graph: Rc<Node> = Node::from_void();
//
//    absorb(Rc::clone(&basis_graph), Rc::clone(&input_tree));
//    log::info!("Done absorbing input tree into basis graph");
//
//    linearize(Rc::clone(&basis_graph));
//    log::info!("Done linearizing basis graph");
//
//    prune(Rc::clone(&basis_graph));
//    log::info!("Done pruning basis graph");
//
//    basis_graph.debug_visualize("basis_graph_pruned");
//    basis_graph.debug_statistics("basis_graph_pruned");
//
//    let metadata = get_tree_metadata(Rc::clone(&basis_graph)).await;
//    log::debug!("metadata: {:?}", metadata);
//
//    interpret(Rc::clone(&basis_graph), Rc::clone(&output_tree)).await;
//    log::info!("Done interpreting basis graph");
//
//    panic!("abort");
//    
//    log::info!("Harvesting output tree...");
//
//    Traversal::from_tree(output_tree)
//        .with_basis(basis_graph)
//        .with_metadata(metadata)
//        .harvest()
//}
