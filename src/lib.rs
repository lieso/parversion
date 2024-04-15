use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::rc::{Rc};

mod models;
mod utilities;
mod trees;
mod llm;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
}

pub fn string_to_json(raw_document: String) -> Result<models::Output, Errors> {
    log::trace!("In string_to_json");

    if raw_document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let document = raw_document.trim().to_string();

    let _ = Runtime::new().unwrap().block_on(async {

        if utilities::is_valid_html(&document) {
            log::info!("Document is valid HTML");

            let xhtml = utilities::html_to_xhtml(&document).expect("Could not convert HTML to XHTML");

            let json = xml_to_json(&xhtml).await?;

            return Ok(json);
        }

        if utilities::is_valid_xml(&document) {
            log::info!("Document is valid XML");

            let json = xml_to_json(&document).await?;

            return Ok(json);
        }

        Err(Errors::UnexpectedDocumentType)
    });

    Err(Errors::UnexpectedError)
}

pub async fn xml_to_json(xml_string: &str) -> Result<Output, Errors> {
    log::trace!("In xml_to_json");

    let xml = utilities::preprocess_xml(xml_string);
    let input_tree = trees::build_tree(xml.clone());

    let basis_tree: Rc<Node> = get_basis_tree();

    apply_basis_tree(Rc:clone(basis_tree), Rc:clone(input_tree));

    basis_tree.harvest()
}

pub fn get_basis_tree() -> Rc<Node> {
    Node::from_void()
}



pub fn apply_basis_tree(basis_tree: Rc<Node>, input_tree: Rc<Node>) {


    while !input_tree.children.is_empty() {

        let mut basis_node = basis_tree;

        trees::dfs(input_tree.clone(), &mut |input_node: &Rc<Node>|) {


            let basis_child = basis_node.children.find(|&&x| x.equals(input_node));

            if let Some(basis_child) {

                basis_node = basis_child;

            } else {

                basis_node = basis_node.adopt_child(input_node);

            }
            
            


        });

    }

}




//pub fn apply_basis_tree(basis_tree: Rc<Node>, input_tree: Rc<Node>) {
//
//    while !input_tree.children.is_empty() {
//
//        basis_tree = basis_tree.navigate_to_root();
//
//        trees::dfs(input_tree.clone(), &mut |input_node: &Rc<Node>| {
//
//            trees::dfs(Rc::clone(basis_tree)), &mut |basis_node: &Rc<Node>| {
//
//
//
//            });
//
//            for basis_child in basis_tree.children {
//                if basis_child.subtree_hash == node.subtree_hash {
//                    basis_child::consume_matching_subtree(node);
//
//                    exit
//                }
//            }
//
//
//            node.remove_from_parent();
//            let mut adopted_node = node;
//            adopted_node.parent = Some(Weak(basis_tree));
//            basis_tree.children.push(adopted_node);
//            basis_tree update subtree hashes
//
//
//            exit
//        });
//    }
//}

//pub async fn basis_tree_from_input_tree(tree: Rc<Node>) -> Rc<Node> {
//    log::trace!("In basis_tree_from_input_tree");
//    trees::log_tree(tree.clone(), "@Pristine");
//
//    trees::merge_recurring_subtrees(Rc::clone(&tree)); // where immediate descendants do not match?
//    trees::log_tree(tree.clone(), "@Merged");
//
//    let unique_subtrees = trees::update_subtree_hashes(Rc::clone(&tree));
//    trees::log_tree(tree.clone(), "@Hashed");
//
//    trees::prune_tree(Rc::clone(&tree), &unique_subtrees);
//    trees::log_tree(tree.clone(), "@Pruned");
//
//    trees::grow_tree(Rc::clone(&tree)).await;
//    trees::log_tree(tree.clone(), "@Populated");
//
//    tree
//}






pub fn file_to_json(file_name: &str) -> Result<i8, Errors> {
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
