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
pub struct NetworkTransformationResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkInferenceResponse {
    pub name: String,
    pub description: String,
    pub fields: Vec<String>,
    pub cardinality: String,
    pub field_types: Vec<String>,
    pub context: String,
    pub structure: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkInferenceResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkTransformationResponse {
    pub data: Option<NetworkInferenceResponse>,
    pub metadata: NetworkTransformationResponseMetadata,
}

pub struct NetworkAnalysis;

impl NetworkAnalysis {
    pub async fn get_network_transformation(
        json_snippet: &str,
        document_summary: &str,
    ) -> Result<NetworkTransformationResponse, Errors> {
        log::trace!("In get_network_transformation");

        let system_prompt = format!(r##"
You are an expert data engineer reverse-engineering a backend data model from rendered HTML JSON structures.

### Document Context:
Website Summary: {}

### Goal:
Analyze the provided JSON structure (which may contain hash keys) and infer the semantic entity it represents, along with comprehensive metadata about its structure and contents.

### Instructions:
1. **Name**: Create a semantically accurate `snake_case` name for this entity/network that reflects what it represents in the data model.
2. **Description**: Write a concise description of what this entity represents, as if writing documentation for an API schema.
3. **Fields**: Extract and list all field names present in the JSON (convert hash keys to descriptive names if needed).
4. **Cardinality**: Determine if this represents a single entity instance or a collection/array of entities ("single" or "collection").
5. **Field Types**: Infer the primitive types of the fields (string, number, boolean, url, datetime, object, array).
6. **Context**: Identify the semantic context or category (e.g., "user_profile", "product_listing", "comment_thread", "transaction_record").
7. **Structure**: Classify the structure type ("flat", "nested", or "hierarchical").

### Response Format:
Respond with valid JSON:
{{
  "name": "string",
  "description": "string",
  "fields": ["field1", "field2", ...],
  "cardinality": "single|collection",
  "field_types": ["string", "number", ...],
  "context": "string",
  "structure": "flat|nested|hierarchical"
}}
"##, document_summary);

        let user_prompt = format!(r##"
[JSON Structure]
{}
"##, json_snippet);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       INFER NETWORK TRANSFORMATION - LLM REQUEST              ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");
        log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
        log::debug!("{}", system_prompt);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");
        log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
        log::debug!("{}", user_prompt);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");

        Self::send_network_inference_request(&system_prompt, &user_prompt).await
    }

    async fn send_network_inference_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<NetworkTransformationResponse, Errors> {
        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "network_inference",
            true,
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The inferred snake_case entity name"
                    },
                    "description": {
                        "type": "string",
                        "description": "Concise description of the entity"
                    },
                    "fields": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of field names in this entity"
                    },
                    "cardinality": {
                        "type": "string",
                        "enum": ["single", "collection"],
                        "description": "Whether this represents a single entity or a collection"
                    },
                    "field_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Primitive types of the fields"
                    },
                    "context": {
                        "type": "string",
                        "description": "Semantic context or category of this entity"
                    },
                    "structure": {
                        "type": "string",
                        "enum": ["flat", "nested", "hierarchical"],
                        "description": "The structural complexity of this entity"
                    }
                },
                "required": ["name", "description", "fields", "cardinality", "field_types", "context", "structure"],
                "additionalProperties": false
            }),
        );

        let request = ChatCompletionRequest::builder()
            .model("gpt-5-mini")
            .messages(vec![
                Message::new(Role::System, system_prompt),
                Message::new(Role::User, user_prompt),
            ])
            .response_format(response_format)
            .build()
            .expect("Could not create llm request");

        match client.send_chat_completion(&request).await {
            Ok(response) => {
                log::debug!("┌─── RAW LLM RESPONSE ──────────────────────────────────────────┐");
                log::debug!("{:?}", response);
                log::debug!("└───────────────────────────────────────────────────────────────┘");
                log::debug!("");

                if let Some(content) = response.choices[0].content() {
                    log::debug!(
                        "┌─── LLM RESPONSE CONTENT ──────────────────────────────────────┐"
                    );
                    log::debug!("{}", content);
                    log::debug!(
                        "└───────────────────────────────────────────────────────────────┘"
                    );
                    log::debug!("");

                    let inference_response = {
                        match serde_json::from_str::<NetworkInferenceResponse>(content) {
                            Ok(parsed_response) => {
                                log::debug!(
                                    "┌─── PARSED RESPONSE ───────────────────────────────────────────┐"
                                );
                                log::debug!("{:?}", parsed_response);
                                log::debug!(
                                    "└───────────────────────────────────────────────────────────────┘"
                                );
                                log::debug!("");
                                Ok(parsed_response)
                            }
                            Err(e) => {
                                log::error!("Failed to parse LLM response: {}", e);
                                Err(Errors::UnexpectedError)
                            }
                        }
                    }?;

                    let metadata = {
                        if let Some(usage) = response.usage {
                            NetworkInferenceResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            NetworkInferenceResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok(NetworkTransformationResponse {
                        data: Some(inference_response),
                        metadata: NetworkTransformationResponseMetadata {
                            tokens: metadata.tokens,
                        }
                    })
                } else {
                    log::error!("No content in LLM response");
                    Err(Errors::UnexpectedError)
                }
            }
            Err(e) => {
                log::error!("Failed to get response from OpenRouter: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
