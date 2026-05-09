use std::collections::HashMap;
use xmltree::{Element, XMLNode};

use crate::prelude::*;

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
            }
            XMLNode::Text(text_node) => (text_node.to_string(), None),
            _ => panic!("Unexpected XML node type"),
        }
    }

    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        let (a, b) = self.to_string_components();

        format!("{}{}", a, b.unwrap_or("".to_string()))
    }

    pub fn get_fields(&self) -> HashMap<String, String> {
        match &self.data {
            XMLNode::Element(element_node) => {
                let mut fields = element_node.attributes.clone();
                fields
            }
            XMLNode::Text(text_node) => {
                HashMap::from([("text".to_string(), text_node.trim().to_string())])
            }
            _ => panic!("Unexpected XML node type"),
        }
    }

    pub fn get_attribute_value(&self, attribute: &str) -> Option<String> {
        match &self.data {
            XMLNode::Element(element_node) => {
                log::debug!("element_node.attributes: {:?}", element_node.attributes);
                element_node.attributes.get(attribute).cloned()
            }
            _ => panic!("Unexpected XML node type"),
        }
    }

    pub fn get_description(&self) -> String {
        match &self.data {
            XMLNode::Element(element_node) => element_node.name.clone(),
            XMLNode::Text(text_node) => {
                let mut description = text_node.to_string();

                let truncate_at = description
                    .char_indices()
                    .nth(20)
                    .map_or(description.len(), |(idx, _)| idx);
                description.truncate(truncate_at);

                description
            }
            _ => panic!("Unexpected XML node type"),
        }
    }

    pub fn get_children(&self) -> Vec<DocumentNode> {
        match &self.data {
            XMLNode::Element(element_node) => element_node
                .children
                .iter()
                .map(|child| DocumentNode::new(child.clone()))
                .collect(),
            XMLNode::Text(_text_node) => Vec::new(),
            _ => panic!("Unexpected XML node type"),
        }
    }

    pub fn get_element_name(&self) -> String {
        match &self.data {
            XMLNode::Element(element_node) => element_node.name.clone(),
            XMLNode::Text(_) => "#text".to_string(),
            _ => panic!("Unexpected XML node type"),
        }
    }

    pub fn get_hash(&self) -> Hash {
        match &self.data {
            XMLNode::Element(element_node) => {
                let mut attr_names: Vec<&String> = element_node.attributes.keys().collect();
                attr_names.sort();

                let attr_str = attr_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
                let combined = format!("{}:{}", element_node.name, attr_str);
                Hash::from_str(&combined)
            }
            XMLNode::Text(_) => Hash::from_str("text"),
            _ => panic!("Unexpected XML node type"),
        }
    }

    fn get_opening_tag(element: &Element) -> String {
        let mut tag = format!("<{}", element.name);

        let mut attributes: Vec<(&String, &String)> = element.attributes.iter().collect();

        // Sort to ensure deterministic hashing
        attributes.sort_by(|a, b| a.0.cmp(b.0));

        for (attr, value) in attributes {
            let fixed_value = {
                if is_valid_url(value) {
                    return shorten_url(value);
                }

                value
            };

            let fixed_value: String = fixed_value.chars().take(100).collect();

            tag.push_str(&format!(
                " {}=\"{}\"",
                attr,
                fixed_value.replace("\"", "&quot;")
            ));
        }
        tag.push('>');

        tag
    }

    fn get_closing_tag(element: &Element) -> String {
        format!("</{}>", element.name)
    }
}
