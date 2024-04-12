extern crate simple_logging;
extern crate log;

use sha2::{Sha256, Digest};
use crate::models::*;
use crate::utilities;
use crate::llm;
use xmltree::{Element, XMLNode};
use async_recursion::async_recursion;
use sled::Db;
use std::io::Cursor;
use std::collections::HashMap;

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
    node.obtain_node_data(&db).await.expect("Unable to obtain data for a Node");

    for child in &node.children {
        traverse_and_populate(&db, &mut child.clone()).await;
    }
}

impl NodeData {
    pub fn new() -> Self {
        NodeData {
            xpath: None,
            variants: Vec::new(),
            is_url: false,
            value: None,
        }
    }

    pub fn peek(&self) -> Option<&str> {
        self.variants.last().map(|x| x.as_str())
    }

    pub fn push(&mut self, variant: String) {
        self.variants.push(variant)
    }
}

impl Node {
    pub fn generate_values(&mut self) {
        let recomputed_node_data = self.data.iter().map(|item| {
            let mut copy = item.clone();

            if let Some(xpath) = copy.xpath.clone() {
                copy.value = Some(
                    utilities::apply_xpath(&self.xml, &xpath)
                        .expect("Could not apply xpath to xml")
                );
            }

            return copy;
        }).collect();

        self.data = recomputed_node_data;
    }

    pub fn to_simple_json(&self) -> serde_json::Value {
        let mut data: HashMap<String, String> = HashMap::new();

        for node_data in self.data.iter() {
            let simple_key = node_data.peek().expect("Could not peek at last key").to_string();
            let value = node_data.value.as_ref().unwrap_or(&String::new()).clone();

            data.insert(simple_key, value);
        }

        serde_json::to_value(data).expect("Failed to convert map to JSON")
    }
    
    pub fn from_element(element: &Element) -> Self {
        let xml = utilities::get_element_tag(&element);
        let hash = Self::compute_node_hash(xml.clone());

        let mut node = Node {
            hash: hash.clone(),
            xml: xml,
            data: Vec::new(),
            children: Vec::new(),
        };

        for child in &element.children {
            if let XMLNode::Element(child_element) = child {
                node.children.push(Node::from_element(&child_element));
            }
        }

        node
    }

    pub fn compute_node_hash(text: String) -> String {
        let mut hasher = Sha256::new();

        hasher.update(text);

        let result = hasher.finalize();
        
        format!("{:x}", result)
    }

    pub async fn obtain_node_data(&mut self, db: &Db) -> Result<(),()> {

        // TODO: detect if blank tag without attributes and skip

        if let Some(node_data) = utilities::get_node_data(&db, &self.hash).expect("Could not obtain node data") {
            self.data = node_data.clone();
        } else {
            let llm_node_data: Vec<NodeData> = llm::generate_node_data(self.xml.clone()).await.expect("LLM unable to generate node data");
            self.data = llm_node_data.clone();

            utilities::store_node_data(&db, &self.hash, llm_node_data.clone()).expect("Unable to persist node data to database");
        }

        self.generate_values();

        Ok(())
    }
}
