use serde::{Deserialize, Serialize};
use crate::node_data::{NodeData};
use crate::node_data_structure::{RecursiveStructure};
use crate::config::{CONFIG};
use crate::constants::{LlmProvider};
use crate::macros::*;
use crate::json_schema::{SchemaMapping};

mod openai;
mod anthropic;
mod groq;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LLMPageClassificationResponse {
    pub interface_type_id: String,
    pub name: String,
    pub core_purpose: String,
    pub has_recursive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LLMSchemaMapping {
    pub source: String,
    pub target: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LLMSchemaMappingResponse {
    pub mappings: Vec<LLMSchemaMapping>,
}

pub async fn get_schema_mapping(schema_from: String, schema_to: String) -> LLMSchemaMappingResponse {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::get_schema_mapping(schema_from, schema_to).await,
        LlmProvider::anthropic => unimplemented!(),
        LlmProvider::groq => unimplemented!(),
    }
}

pub async fn get_interface_type(page: String) -> LLMPageClassificationResponse {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::get_interface_type(page).await,
        LlmProvider::anthropic => unimplemented!(),
        LlmProvider::groq => unimplemented!(),
    }
}

pub async fn interpret_associations(snippets: Vec<(String, String)>) -> Vec<Vec<String>> {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::interpret_associations(snippets).await,
        LlmProvider::anthropic => anthropic::interpret_associations(snippets).await,
        LlmProvider::groq => groq::interpret_associations(snippets).await,
    }
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> RecursiveStructure {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::interpret_data_structure(snippets).await,
        LlmProvider::anthropic => anthropic::interpret_data_structure(snippets).await,
        LlmProvider::groq => groq::interpret_data_structure(snippets).await,
    }
}

pub async fn interpret_element_data(
    meaningful_attributes: Vec<String>,
    snippets: Vec<String>,
    core_purpose: String
) -> Vec<NodeData> {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::interpret_element_data(meaningful_attributes, snippets, core_purpose).await,
        LlmProvider::anthropic => anthropic::interpret_element_data(meaningful_attributes, snippets, core_purpose).await,
        LlmProvider::groq => groq::interpret_element_data(meaningful_attributes, snippets, core_purpose).await,
    }
}

pub async fn interpret_text_data(snippets: Vec<String>, core_purpose: String) -> NodeData {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::interpret_text_data(snippets, core_purpose).await,
        LlmProvider::anthropic => anthropic::interpret_text_data(snippets, core_purpose).await,
        LlmProvider::groq => groq::interpret_text_data(snippets, core_purpose).await,
    }
}

fn get_llm_provider() -> LlmProvider {
    read_lock!(CONFIG).llm.llm_provider.clone()
}

