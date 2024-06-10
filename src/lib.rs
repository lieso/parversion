use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::rc::{Rc};

mod error;
mod llm;
mod node;
mod node_data;
mod traversal;
mod utility;
mod xml;
mod vision;

use node::{
    Node,
    build_tree,
    absorb_tree,
    prune_tree,
    grow_tree,
    collapse_linear_nodes
};
use error::{Errors};
use traversal::{Traversal};

pub fn string_to_json(raw_document: String) -> Result<String, Errors> {
    log::trace!("In string_to_json");

    if raw_document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let document = raw_document.trim().to_string();

    return Runtime::new().unwrap().block_on(async {

        if utility::is_valid_html(&document) {
            log::info!("Document is valid HTML");




            //let color_palette = vision::html_to_color_palette(document.clone()).await;

            //unimplemented!();






            let xhtml = utility::html_to_xhtml(&document).expect("Could not convert HTML to XHTML");

            let json = xml_to_json(&xhtml).await?;

            return Ok(json);
        }

        if utility::is_valid_xml(&document) {
            log::info!("Document is valid XML");

            let json = xml_to_json(&document).await?;

            return Ok(json);
        }

        Err(Errors::UnexpectedDocumentType)
    });
}

pub async fn xml_to_json(xml_string: &str) -> Result<String, Errors> {
    log::trace!("In xml_to_json");

    let xml = utility::preprocess_xml(xml_string);
    log::info!("Done preprocessing XML");

    let input_tree: Rc<Node> = build_tree(xml.clone());
    //let output_tree: Rc<Node> = Rc::new((*input_tree).clone());
    let output_tree: Rc<Node> = build_tree(xml.clone());

    log::info!("Done building input/output trees");

    let basis_tree: Rc<Node> = get_basis_tree();
    log::info!("Obtained basis tree with subtree hash: {}", basis_tree.subtree_hash());

    absorb_tree(Rc::clone(&basis_tree), Rc::clone(&input_tree));
    log::info!("Done absorbing input tree into basis tree");

    prune_tree(Rc::clone(&basis_tree));
    log::info!("Done pruning basis tree");

    collapse_linear_nodes(Rc::clone(&basis_tree));
    collapse_linear_nodes(Rc::clone(&output_tree));
    log::info!("Done collapsing linear nodes");

    grow_tree(Rc::clone(&basis_tree)).await;
    log::info!("Done growing basis tree");

    save_basis_tree(Rc::clone(&basis_tree));
    log::info!("Saved basis tree");

    log::info!("Beginning traversal of output tree...");

    Traversal::from_tree(output_tree)
        .with_basis(basis_tree)
        .traverse()?
        .harvest()
}

pub fn get_basis_tree() -> Rc<Node> {
    Node::from_void()
}

pub fn save_basis_tree(tree: Rc<Node>) {
    log::warn!("save_basis_tree unimplemented");
}

pub fn file_to_json(file_name: &str) -> Result<String, Errors> {
    log::trace!("In file_to_json");
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

    return string_to_json(document);
}
