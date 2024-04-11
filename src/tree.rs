use xmltree::{Element, XMLNode};
use std::collections::HashMap;
use std::io::Read;
use sha2::{Sha256, Digest};
use serde_json::{self, Value};
use std::io::Cursor;
use serde::{Serialize, Deserialize};
use async_recursion::async_recursion;

#[derive(Debug)]
enum Errors {
    UnexpectedError
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    xpath: Option<String>,
    variants: Vec<String>,
    is_url: bool,
    value: Option<String>,
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
    hash: String,
    xml: String,
    start_tag: String,
    data: Vec<NodeData>,
    children: Vec<Node>
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

    pub async fn obtain_data(&mut self) -> Result<(), Errors> {

        Ok(())
    }
}

pub fn build_tree(xml: String) -> Node {
   let mut reader = std::io::Cursor::new(xml);
   let element = Element::parse(&mut reader).expect("Could not parse XML");

   Node::from_element(&element)
}

pub async fn grow_tree(tree: &mut Node) -> Node {
    traverse_and_populate(tree).await;

    log::debug!("tree: {:?}", tree);

    tree.clone()
}

#[async_recursion]
async fn traverse_and_populate(node: &mut Node) {
    node.obtain_data().await.expect("Unable to obtain data for a Node");

    for child in &node.children {
        traverse_and_populate(&mut child.clone()).await;
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
