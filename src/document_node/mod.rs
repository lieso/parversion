use crate::prelude::*;
use crate::data_node::DataNodeFields;

mod xml;
mod json;

use xml::Xml;
use json::Json;

#[derive(Clone, Debug)]
pub enum DocumentNodeData {
    Xml(xmltree::XMLNode),
    Json(serde_json::Map<String, serde_json::Value>),
}

#[derive(Clone, Debug)]
pub struct DocumentNode {
    pub id: ID,
    data: DocumentNodeData,
}

impl DocumentNode {
    pub fn new(data: DocumentNodeData) -> Self {
        DocumentNode {
            id: ID::new(),
            data,
        }
    }

    pub fn to_string(&self) -> String {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::to_string(&node),
            DocumentNodeData::Json(map) => Json::to_string(map),
        }
    }

    pub fn to_string_components(&self) -> (String, Option<String>) {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::to_string_components(&node),
            DocumentNodeData::Json(value) => panic!("Unexpected DocumentNodeData"),
        }
    }

    pub fn get_fields(&self) -> DataNodeFields {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_fields(&node),
            DocumentNodeData::Json(map) => Json::get_fields(map),
        }
    }

    pub fn get_attribute_value(&self, attribute: &str) -> Option<String> {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_attribute_value(&node, attribute),
            DocumentNodeData::Json(value) => panic!("Unexpected DocumentNodeData"),
        }
    }
    
    pub fn get_description(&self) -> String {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_description(&node),
            DocumentNodeData::Json(value) => Json::get_description(&value),
        }
    }
    
    pub fn get_children(&self) -> Vec<DocumentNode> {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_children(&node)
                .into_iter()
                .map(|xml_node| DocumentNode::new(DocumentNodeData::Xml(xml_node)))
                .collect(),
            DocumentNodeData::Json(map) => Json::get_children(map)
                .into_iter()
                .map(|child_map| DocumentNode::new(DocumentNodeData::Json(child_map)))
                .collect(),
        }
    }
    
    pub fn get_element_name(&self) -> String {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_element_name(&node),
            DocumentNodeData::Json(value) => panic!("Unexpected DocumentNodeData"),
        }
    }

    pub fn get_hash(&self) -> Hash {
        match &self.data {
            DocumentNodeData::Xml(node) => Xml::get_hash(&node),
            DocumentNodeData::Json(map) => Json::get_hash(map),
        }
    }
}
