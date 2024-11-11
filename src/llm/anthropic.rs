#![allow(warnings)]
use crate::node_data_structure::{RecursiveStructure};
use crate::node_data::{NodeData, ElementData, TextData};

pub async fn interpret_element_data(meaningful_attributes: Vec<String>, snippets: Vec<String>, core_purpose: String) -> Vec<NodeData> {
    log::trace!("In interpret_element_data");
    unimplemented!()
}

pub async fn interpret_text_data(snippets: Vec<String>, core_purpose: String) -> NodeData {
    log::trace!("In interpret_text_data");
    unimplemented!()
}

pub async fn summarize_core_purpose(xml: String) -> String {
    log::trace!("In summarize_core_purpose");
    unimplemented!()
}

pub async fn interpret_associations(snippets: Vec<(String, String)>) -> Vec<Vec<String>> {
    log::trace!("In interpret_associations");
    unimplemented!()
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> RecursiveStructure {
    log::trace!("In interpret_data_structure");
    unimplemented!()
}
