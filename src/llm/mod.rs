use serde::{Deserialize, Serialize};
use parversion::node_data::{NodeData};
use parversion::node_data_structure::{RecursiveStructure};
use parversion::config::{CONFIG};
use parversion::constants::{LlmProvider};
use parversion::macros::*;

mod openai;
mod anthropic;
mod groq;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LLMWebsiteAnalysisResponse {
    pub core_purpose: String,
    pub has_recursive: bool,
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

pub async fn analyze_compressed_website(xml: String) -> LLMWebsiteAnalysisResponse {
    let llm_provider = get_llm_provider();

    match llm_provider {
        LlmProvider::openai => openai::analyze_compressed_website(xml).await,
        LlmProvider::anthropic => anthropic::analyze_compressed_website(xml).await,
        LlmProvider::groq => groq::analyze_compressed_website(xml).await,
    }
}

fn get_llm_provider() -> LlmProvider {
    read_lock!(CONFIG).llm.llm_provider.clone()
}

