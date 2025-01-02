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

    pub fn get_fields(&self) -> HashMap<String, String> {
        match &self.data {
            XMLNode::Element(element_node) => element_node.attributes.clone(),
            XMLNode::Text(text_node) => HashMap::from([
                ("text".to_string(), text_node.to_string())
            ]),
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_description(&self) -> String {
        match &self.data {
            XMLNode::Element(element_node) => {
                let mut description = format!("{:?}", element_node);
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
}
