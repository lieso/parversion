use xmltree::{XMLNode, Element};
use std::collections::HashMap;

use crate::prelude::*;
use crate::transformation::XMLElementTransformation;

#[derive(Clone, Debug)]
pub struct DocumentNode {
    pub id: ID,
    data: XMLNode,
}

impl DocumentNode {
    pub fn new(xml_node: XMLNode) -> Self {
        DocumentNode {
            id: ID::new(),
            data: xml_node.clone(),
        }
    }

    pub fn to_string_components(&self) -> (String, Option<String>) {
        match &self.data {
            XMLNode::Element(element_node) => {
                let opening_tag = DocumentNode::get_opening_tag(&element_node);
                let closing_tag = DocumentNode::get_closing_tag(&element_node);

                (opening_tag, Some(closing_tag))
            },
            XMLNode::Text(text_node) => {
                (text_node.to_string(), None)
            },
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn to_string(&self) -> String {
        let (a, b) = self.to_string_components();

        format!("{}{}", a, b.unwrap_or("".to_string()))
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

                let truncate_at = description.char_indices().nth(20).map_or(description.len(), |(idx, _)| idx);
                description.truncate(truncate_at);

                description
            },
            _ => panic!("Unexpected XML node type")
        }
    }

    pub fn get_children(
        &self,
        xml_element_transformation: Option<XMLElementTransformation>
    ) -> Vec<DocumentNode> {
        match &self.data {
            XMLNode::Element(element_node) => {
                element_node.children
                    .iter()
                    .filter_map(|child| {

                        if let Some(xml_element_transformation) = &xml_element_transformation {
                            DocumentNode::from_transformations(
                                child.clone(),
                                xml_element_transformation.clone()
                            )
                        } else {
                            Some(DocumentNode::new(child.clone()))
                        }
                    })
                    .collect()
            },
            XMLNode::Text(_text_node) => Vec::new(),
            _ => panic!("Unexpected XML node type")
        }
    }

    fn get_opening_tag(element: &Element) -> String {
        let mut tag = format!("<{}", element.name);

        let mut attributes: Vec<(&String, &String)> = element.attributes.iter().collect();

        // Sort to ensure deterministic hashing
        attributes.sort_by(|a, b| a.0.cmp(b.0));

        for (attr, value) in attributes {
            tag.push_str(&format!(" {}=\"{}\"", attr, value.replace("\"", "&quot;")));
        }
        tag.push('>');

        tag
    }

    fn get_closing_tag(element: &Element) -> String {
        format!("</{}>", element.name)
    }
}
