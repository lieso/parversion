use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prelude::*;
use crate::environment::get_env_variable;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTransformationResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTransformationResponse {
    pub metadata: NodeTransformationResponseMetadata,
}

pub struct NodeAnalysis;

struct EliminationResponseMetadata {
    tokens: u64,
}

struct EliminationResponse {
    should_eliminate: bool,
    metadata: EliminationResponseMetadata,
}

impl NodeAnalysis {

    pub async fn get_node_transformation(
        field: &str,
        value: &str,
        field_snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<NodeTransformationResponse, Errors> {
        log::trace!("In get_node_transformation");

        let mut tokens: u64 = 0;
        
        let should_eliminate_response: EliminationResponse = match field {
            "text" => Self::should_eliminate_text(
                field_snippets.clone(),
                document_summary,
            ).await?,
            _ => Self::should_eliminate_attribute(
                field_snippets.clone(),
                document_summary,
            ).await?,
        };

        if should_eliminate_response.should_eliminate {
            tokens += should_eliminate_response.metadata.tokens;

            let result = NodeTransformationResponse {
                metadata: NodeTransformationResponseMetadata {
                    tokens,
                }
            };

            return Ok(result);
        }

        unimplemented!()
    }

    async fn should_eliminate_text(
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<EliminationResponse, Errors> {
        log::trace!("In should_eliminate_text");

        unimplemented!();
    }

    async fn should_eliminate_attribute(
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<EliminationResponse, Errors> {
        log::trace!("In should_eliminate_attribute");

        unimplemented!();
    }

}
