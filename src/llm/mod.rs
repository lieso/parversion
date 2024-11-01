use crate::node_data::{NodeData};
use crate::node_data_structure::{RecursiveStructure};

mod openai;

pub async fn interpret_associations(snippets: Vec<(String, String)>) -> Vec<Vec<String>> {
    openai::interpret_associations(snippets).await
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> RecursiveStructure {
    openai::interpret_data_structure(snippets).await
}

pub async fn interpret_element_data(meaningful_attributes: Vec<String>, snippets: Vec<String>) -> Vec<NodeData> {
    openai::interpret_element_data(meaningful_attributes, snippets).await
}

pub async fn interpret_text_data(snippets: Vec<String>) -> NodeData {
    openai::interpret_text_data(snippets).await
}
