extern crate simple_logging;
extern crate log;

use crate::models::tree::*;
use xmltree::{Element};
use async_recursion::async_recursion;
use sled::Db;

pub fn build_tree(xml: String) -> Node {
   let mut reader = std::io::Cursor::new(xml);
   let element = Element::parse(&mut reader).expect("Could not parse XML");

   Node::from_element(&element)
}

pub async fn grow_tree(tree: &mut Node) -> Node {
    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    traverse_and_populate(&db, tree).await;

    tree.clone()
}

#[async_recursion]
async fn traverse_and_populate(db: &Db, node: &mut Node) {
    node.obtain_data(&db).await.expect("Unable to obtain data for a Node");

    for child in &node.children {
        traverse_and_populate(&db, &mut child.clone()).await;
    }
}
