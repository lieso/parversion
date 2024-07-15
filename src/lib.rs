use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::rc::{Rc};

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

use node::{
    Node,
    build_tree,
    deep_copy,
    absorb_tree,
    prune_tree,
    grow_tree,
    get_tree_metadata
};
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

    return normalize(document);
}

pub async fn normalize_xml(xml_string: &str) -> Result<String, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(xml_string);
    log::info!("Done preprocessing XML");

    let input_tree: Rc<Node> = build_tree(xml.clone());
    let output_tree: Rc<Node> = deep_copy(&input_tree);

    log::info!("Done building input/output trees");

    let basis_tree: Rc<Node> = get_basis_tree();
    log::info!("Obtained basis tree with subtree hash: {}", basis_tree.subtree_hash());

    absorb_tree(Rc::clone(&basis_tree), Rc::clone(&input_tree));
    log::info!("Done absorbing input tree into basis tree");

    prune_tree(Rc::clone(&basis_tree));
    log::info!("Done pruning basis tree");

    basis_tree.debug_visualize("basis");
    output_tree.debug_visualize("output");

    let metadata = get_tree_metadata(Rc::clone(&basis_tree)).await;
    log::debug!("metadata: {:?}", metadata);

    grow_tree(Rc::clone(&basis_tree), Rc::clone(&output_tree)).await;
    log::info!("Done growing basis tree");

    save_basis_tree(Rc::clone(&basis_tree));
    log::info!("Saved basis tree");

    log::info!("Beginning traversal of output tree...");

    Traversal::from_tree(output_tree)
        .with_basis(basis_tree)
        .with_metadata(metadata)
        .harvest()
}

fn get_basis_tree() -> Rc<Node> {
    Node::from_void()
}

fn save_basis_tree(_tree: Rc<Node>) {
    log::warn!("save_basis_tree unimplemented");
}
