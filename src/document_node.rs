use serde::{Serialize, Deserialize};
use xmltree::{XMLNode};

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DocumentNode {
    data: XMLNode,
}

impl DocumentNode {
    pub fn new(xml_node: XMLNode) -> Self {
        DocumentNode {
            data: xml_node.clone(),
        }
    }

    pub fn from_transformations(
        xml_node: XMLNode,
        transformations: Vec<DocumentTransformation>
    ) -> Self {
        match &xml_node {
            XMLNode::Element(element_node) => {
            },
            XMLNode::Text(text_node) => {
            },
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_fields(&self) -> HashMap<String, String> {
        match &self.data {
            XMLNode::Element(element_node) => {
                let mut fields = element_node.attributes.clone();
                fields.insert("tag".to_string(), element_node.name.clone());
                fields
            }
            XMLNode::Text(text_node) => HashMap::from([
                ("text".to_string(), text_node.to_string())
            ]),
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_description(&self) -> String {
        match &self.data {
            XMLNode::Element(element_node) => {
                let mut description = format!("{}", element_node.to_string());
                description.truncate(20);

                description
            },
            XMLNode::Text(text_node) => {
                let mut description = text_node.to_string();
                description.truncate(20);

                description
            },
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_children(&self, transformations: Vec<DocumentTransformation>) -> Vec<DocumentNode> {
        match &self.data {
            XMLNode::Element(element_node) => {
                element_node.children.iter().map(|child| {
                    DocumentNode::from_transformations(child.clone(), transformations)
                }).collect()
            },
            XMLNode::Text(text_node) => Vec::new(),
            _ => panic!("Unexpected XML node type")
        }
    }
}
