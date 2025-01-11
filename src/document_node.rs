use serde::{Serialize, Deserialize};
use xmltree::{XMLNode};
use std::collections::HashMap;

use crate::transformation::XMLElementTransformation;

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
        xml_element_transformation: XMLElementTransformation,
    ) -> Option<Self> {
        match &xml_node {
            XMLNode::Element(element_node) => {
                let mut element: Option<String> = Some(element_node.name.clone());
                let mut attributes: HashMap<String, String>  = HashMap::new();

                for (attr, val) in element_node.attributes.iter() {
                    attributes.insert(attr.to_string(), val.to_string());
                }

                log::info!("Applying XML element transformation...");

                let (transformed_element, transformed_attributes) = xml_element_transformation.transform(
                    element.unwrap().clone(),
                    attributes.clone()
                );

                attributes = transformed_attributes;

                if let Some(transformed_element) = transformed_element {
                    element = Some(transformed_element);
                } else {
                    log::info!("Transformation has eliminated an element, no further transfomations will be applied");
                    element = None;
                }

                log::info!("Done applying XML element transformations.");

                element.map(|some_element| {
                    let mut transformed_node = xml_node.clone();

                    if let XMLNode::Element(ref mut elem) = transformed_node {
                        elem.name = some_element;
                        elem.attributes = attributes;
                    }

                    DocumentNode::new(transformed_node)
                })
            },
            XMLNode::Text(_text_node) => {
                Some(DocumentNode::new(xml_node))
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
                element_node.name.clone()
            },
            XMLNode::Text(text_node) => {
                let mut description = text_node.to_string();
                description.truncate(20);

                description
            },
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_children(&self, xml_element_transformation: XMLElementTransformation) -> Vec<DocumentNode> {
        match &self.data {
            XMLNode::Element(element_node) => {
                element_node.children
                    .iter()
                    .filter_map(|child| {
                        DocumentNode::from_transformations(
                            child.clone(),
                            xml_element_transformation.clone()
                        )
                    })
                    .collect()
            },
            XMLNode::Text(text_node) => Vec::new(),
            _ => panic!("Unexpected XML node type")
        }
    }
}
