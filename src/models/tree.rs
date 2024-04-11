use sha2::{Sha256, Digest};
use std::collections::HashMap;
use xmltree::{Element, XMLNode};
use serde::{Serialize, Deserialize};
use sled::Db;
use std::io::Cursor;

use crate::database;

#[derive(Debug)]
pub enum Errors {
    UnexpectedError
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub xpath: Option<String>,
    pub variants: Vec<String>,
    pub is_url: bool,
    pub value: Option<String>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub hash: String,
    pub xml: String,
    pub start_tag: String,
    pub data: Vec<NodeData>,
    pub children: Vec<Node>
}

impl Node {
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
        let hash = Self::compute_element_hash(element);

        let mut node = Node {
            hash: hash.clone(),
            xml: element_to_string(&element).unwrap(),
            start_tag: start_tag_to_string(&element),
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

    pub fn compute_element_hash(element: &Element) -> String {
        let mut cursor = Cursor::new(Vec::new());
        element.write(&mut cursor).expect("Could not write cursor");

        let xml_string = String::from_utf8(cursor.into_inner())
            .expect("Found invalid UTF-8");

        let mut hasher = Sha256::new();

        hasher.update(xml_string);

        let result = hasher.finalize();

        format!("{:x}", result)
    }

    pub async fn obtain_data(&mut self, db: &Db) -> Result<(), Errors> {
        if let Some(node_data) = database::get_node_data(&db, &self.hash)
            .expect("Could not obtain node data") {

            self.data = node_data.clone();
        } else {
            return Err(Errors::UnexpectedError);
        }





        Ok(())
    }
}

fn element_to_string(element: &Element) -> Result<String, std::io::Error> {
    let mut cursor = Cursor::new(Vec::new());
    element.write(&mut cursor).expect("Element could not write");
    Ok(String::from_utf8(cursor.into_inner()).expect("Found invalid UTF-8"))
}

fn start_tag_to_string(element: &Element) -> String {
    let attributes = element
        .attributes
        .iter()
        .map(|(k, v)| format!(r#"{}="{}""#, k, v))
        .collect::<Vec<_>>()
        .join(" ");

    let prefix = match element.prefix {
        Some(ref p) => format!("{}:", p),
        None => "".to_string(),
    };

    format!("<{}{}{}>", prefix, element.name, if attributes.is_empty() { "" } else { " " }.to_owned() + &attributes)
}
