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
pub struct TranslateNetworksResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslateNetworksResponse {
    pub is_match: bool,
    pub source_cardinality: String,
    pub target_cardinality: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslateNodesResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeMatch {
    pub source_key: String,
    pub target_key: String,
    pub transform_code: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslateNodesResponse {
    pub matches: Vec<NodeMatch>,
}

pub struct Translation;

impl Translation {
    pub async fn translate_networks(
        user_prompt: &str
    ) -> Result<(TranslateNetworksResponse, TranslateNetworksResponseMetadata), Errors> {
        log::trace!("In translate_networks");

        let system_prompt = r##"
You are an expert data integration engineer specializing in JSON schema mapping and ETL transformations.

Your task is to determine whether a Source network and a Target network represent the same semantic concept, based on their structural position and content.

CONCEPTS:
1. FIRST DOCUMENT (Source): The original data source.
2. SECOND DOCUMENT (Target): The desired final data shape.
3. SPATIAL CONTEXT: An incomplete fragment (a small zoomed-in neighborhood) of the original JSON document centered directly around the network being evaluated. This is provided to save tokens while giving you the actual values and immediate siblings to deduce semantic meaning. (Note: "_omitted": true implies data exists in the original document but was removed for brevity).
4. POSITIONAL CONTEXT: The complete, absolute JSON path from the root of the original document down to the network being evaluated (e.g., "root -> entries -> author"). This provides the full structural lineage of the network.

CRITICAL RULES:
- Use the Positional Context to understand the full structural lineage of each network.
- Use the Spatial Context to analyze the actual values and immediate siblings.
- DO NOT match networks just because they share a similar name or relate to the same broad topic.
- PLURALITY & CARDINALITY: You MUST verify that the data types match conceptually. A collection/array of items DOES NOT match a singular item or a nested properties object.
- GRANULARITY & SCOPE: You MUST verify the hierarchical scope. A top-level container holding multiple attributes DOES NOT match a deeply nested sub-component, even if they share related data.
- Example 1 (Match): A source network at "submissions -> item -> author" and a target network at "entries -> author" DO MATCH if both represent the author of a content item.
- Example 2 (Mismatch - Scope): A source network at "submissions -> metadata" and a target network at "entries -> author" DO NOT MATCH even if both contain a name field.
- Example 3 (Mismatch - Cardinality): A source network at "submissions -> item -> details" (a singular object) and a target network at "entries" (an array of items) DO NOT MATCH. The array maps to the parent array, not the nested item details.

OUTPUT FORMAT:
Return a strictly formatted JSON object with the following keys:
- "source_cardinality": Evaluate the Source network and output either "array", "object", or "primitive" (string/number/boolean).
- "target_cardinality": Evaluate the Target network and output either "array", "object", or "primitive".
- "is_match": Set to true ONLY if the Source and Target networks represent the exact same semantic concept, structural role, AND their cardinalities logically align. Set to false otherwise.

Example Output:
{
  "source_cardinality": "object",
  "target_cardinality": "array",
  "is_match": false
}
        "##;

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║             TRANSLATE NETWORKS - LLM REQUEST                  ║");
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

        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "network_mapping",
            true,
            json!({
                "type": "object",
                "properties": {
                    "is_match": {
                        "type": "boolean",
                        "description": "True if the Source and Target networks represent the same semantic concept and structural role."
                    },
                    "source_cardinality": {
                        "type": "string",
                        "description": "The cardinality of FIRST DOCUMENT network."
                    },
                    "target_cardinality": {
                        "type": "string",
                        "description": "The cardinality of SECOND DOCUMENT network."
                    }
                },
                "required": ["is_match", "source_cardinality", "target_cardinality"],
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

                    let translate_networks_response = {
                        match serde_json::from_str::<TranslateNetworksResponse>(content) {
                            Ok(parsed_response) => {
                                log::debug!(
                                    "┌─── PARSED RESPONSE ───────────────────────────────────────────┐"
                                );
                                log::debug!("{:#?}", parsed_response);
                                log::debug!(
                                    "└───────────────────────────────────────────────────────────────┘"
                                );
                                log::debug!("");
                                log::debug!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "║           TRANSLATE NETWORKS - REQUEST COMPLETE               ║"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );

                                Ok(parsed_response)
                            }
                            Err(e) => {
                                log::error!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::error!(
                                    "║                    PARSE ERROR                                ║"
                                );
                                log::error!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );
                                log::error!("Failed to parse LLM response: {}", e);
                                Err(Errors::UnexpectedError)
                            }
                        }
                    }?;

                    let metadata = {
                        if let Some(usage) = response.usage {
                            TranslateNetworksResponseMetadata {
                                tokens: usage.total_tokens as u64,
                            }
                        } else {
                            TranslateNetworksResponseMetadata {
                                tokens: 0,
                            }
                        }
                    };

                    Ok((translate_networks_response, metadata))
                } else {
                    log::error!(
                        "╔═══════════════════════════════════════════════════════════════╗"
                    );
                    log::error!(
                        "║                    NO CONTENT ERROR                           ║"
                    );
                    log::error!(
                        "╚═══════════════════════════════════════════════════════════════╝"
                    );
                    log::error!("No content in LLM response");
                    Err(Errors::UnexpectedError)
                }
            }
            Err(e) => {
                log::error!("╔═══════════════════════════════════════════════════════════════╗");
                log::error!("║                    REQUEST ERROR                              ║");
                log::error!("╚═══════════════════════════════════════════════════════════════╝");
                log::error!("Failed to get response from OpenRouter: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    pub async fn translate_nodes(
        user_prompt: &str
    ) -> Result<(TranslateNodesResponse, TranslateNodesResponseMetadata), Errors> {
        log::trace!("In translate_nodes");

        let system_prompt = r##"
You are an expert data integration engineer specializing in JSON schema mapping and ETL transformations.

Your task is to compare candidate keys from exactly ONE Source JSON node against ONE Target JSON node, identify ALL semantically equivalent keys, and write data transformation code if the values require formatting changes.

CONCEPTS:
1. FIRST DOCUMENT (Source): The original data source.
2. SECOND DOCUMENT (Target): The desired final data shape.
3. SPATIAL CONTEXT: An incomplete fragment (a small zoomed-in neighborhood) of the original JSON document centered directly around the node being evaluated. This is provided to save tokens while giving you the actual values and immediate siblings to deduce semantic meaning. (Note: "_omitted": true implies data exists in the original document but was removed for brevity).
4. POSITIONAL CONTEXT: The complete, absolute JSON path from the root of the original document down to the candidate keys being evaluated (e.g., "root -> entries -> author -> url"). This provides the full structural lineage of the keys.

CRITICAL RULES FOR DETERMINING A MATCH:
- Combine contexts: Use the Positional Context to understand the full structural lineage of the key, and use the Spatial Context to analyze its actual value and immediate siblings.
- DO NOT blindly map keys just because they share the same name.
- You MUST analyze the SPATIAL CONTEXT (the actual values) to prove that the two fields represent the exact same real-world entity.
- Example: If the Source Positional Context ends in "url" (value: "github.com/user") but the Target Positional Context ends in "url" (value: "example.com/article"), these DO NOT MATCH because one is an author profile and the other is an article link.
- Example: If the Source ends in "submitted_at" (value: "2025-10-28T13:22") and the Target ends in "timestamp" (value: 1746280800), these DO MATCH because the values prove they represent the same publication time.

INSTRUCTIONS:
1. Evaluate the Candidate Keys found at the end of the Positional Context paths from the First Document against those from the Second Document.
2. Identify ALL valid semantic matches based on the rules above.
3. For each match, output ONLY the final `source_key` and `target_key` (e.g., output "url", NOT the full positional path).
4. For each match, examine the values in the Spatial Context. If the data formats differ, you MUST write a pure JavaScript function to convert the source value to the target format.
5. The JavaScript code must be a valid, standalone function named `transform` that takes a single parameter `value` and returns the converted result.

EXAMPLE JAVASCRIPT:
```javascript
function transform(value) {
    return Math.floor(new Date(value).getTime() / 1000);
}
```

If the values are already in the exact same format and type, `transform_code` should be null. If no valid semantic matches exist between the two objects, return an empty array `[]` for matches.
        "##;

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║              TRANSLATE NODES - LLM REQUEST                    ║");
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

        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "node_key_mapping",
            true,
            json!({
                "type": "object",
                "properties": {
                    "matches": {
                        "type": "array",
                        "description": "List of all semantically matched keys between the Source and Target nodes.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "source_key": {
                                    "type": "string",
                                    "description": "The exact key name from the FIRST DOCUMENT node."
                                },
                                "target_key": {
                                    "type": "string",
                                    "description": "The exact key name from the SECOND DOCUMENT node."
                                },
                                "transform_code": {
                                    "type": ["string", "null"],
                                    "description": "A standalone JS function named `transform(value)`. Null if no conversion is needed."
                                }
                            },
                            "required": ["source_key", "target_key", "transform_code"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["matches"],
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

                    let translate_nodes_response = {
                        match serde_json::from_str::<TranslateNodesResponse>(content) {
                            Ok(parsed_response) => {
                                log::debug!(
                                    "┌─── PARSED RESPONSE ───────────────────────────────────────────┐"
                                );
                                log::debug!("{:#?}", parsed_response);
                                log::debug!(
                                    "└───────────────────────────────────────────────────────────────┘"
                                );
                                log::debug!("");
                                log::debug!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "║            TRANSLATE NODES - REQUEST COMPLETE                 ║"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );

                                Ok(parsed_response)
                            }
                            Err(e) => {
                                log::error!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::error!(
                                    "║                    PARSE ERROR                                ║"
                                );
                                log::error!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );
                                log::error!("Failed to parse LLM response: {}", e);
                                Err(Errors::UnexpectedError)
                            }
                        }
                    }?;

                    let metadata = {
                        if let Some(usage) = response.usage {
                            TranslateNodesResponseMetadata {
                                tokens: usage.total_tokens as u64,
                            }
                        } else {
                            TranslateNodesResponseMetadata {
                                tokens: 0,
                            }
                        }
                    };

                    Ok((translate_nodes_response, metadata))
                } else {
                    log::error!(
                        "╔═══════════════════════════════════════════════════════════════╗"
                    );
                    log::error!(
                        "║                    NO CONTENT ERROR                           ║"
                    );
                    log::error!(
                        "╚═══════════════════════════════════════════════════════════════╝"
                    );
                    log::error!("No content in LLM response");
                    Err(Errors::UnexpectedError)
                }
            }
            Err(e) => {
                log::error!("╔═══════════════════════════════════════════════════════════════╗");
                log::error!("║                    REQUEST ERROR                              ║");
                log::error!("╚═══════════════════════════════════════════════════════════════╝");
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
