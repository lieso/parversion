#![allow(warnings)]
use parversion::node_data_structure::{RecursiveStructure};
use parversion::node_data::{NodeData, ElementData, TextData};
use super::{LLMWebsiteAnalysisResponse};

pub async fn interpret_element_data(meaningful_attributes: Vec<String>, snippets: Vec<String>, core_purpose: String) -> Vec<NodeData> {
    log::trace!("In interpret_element_data");
    unimplemented!()
}

pub async fn interpret_text_data(snippets: Vec<String>, core_purpose: String) -> NodeData {
    log::trace!("In interpret_text_data");
    unimplemented!()
}

pub async fn analyze_compressed_website(xml: String) -> LLMWebsiteAnalysisResponse {
    log::trace!("In analyze_compressed_website");
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
