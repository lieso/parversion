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
pub struct EliminatedNetwork {
    pub id: String,
    pub maps_to: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RedundantNetworksResponse {
    pub canonical: Vec<String>,
    pub eliminated: Vec<EliminatedNetwork>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RedundantNetworksResponseMetadata {
    pub tokens: u64,
}

pub struct NetworkRelationships;

impl NetworkRelationships {
    pub async fn infer_redundant_networks(
        original_document: &str,
        all_network_jsons: &str,
    ) -> Result<(RedundantNetworksResponse, RedundantNetworksResponseMetadata), Errors> {
        log::trace!("In infer_redundant_networks");

        let system_prompt = r##"
You are given a set of JSON networks extracted from an HTML document. Each network has a unique ID and one or more examples showing the JSON keys and values that were observed together
in the DOM.

Your task is to deduplicate this set of networks so that each remaining network represents a distinct resource. Perform the following two operations:

1. Remove nested networks
A nested network is one whose structure appears as a subtree within another network's examples. If network A's structure appears embedded inside network B, remove A and map it to B.

2. Remove duplicate networks
A duplicate network represents the same resource as another network but differs in completeness, key naming variation, or number of examples. When two networks represent the same
resource, keep one and eliminate the other.

When deciding which network to keep, apply these criteria in order:
- Prefer the network with the most consistent structure across its examples
- Prefer the network that has more examples
- Prefer the flatter structure when nesting adds no semantic meaning
- Prefer the network whose keys most directly name the data they contain

Output
Respond with valid JSON in the following format:
{
  "canonical": ["id1", "id2", ...],
  "eliminated": [
    { "id": "eliminated_id", "maps_to": "canonical_id", "reason": "one sentence" },
    ...
  ]
}

Do not include any explanation outside the JSON.

---
"##;

        let user_prompt = format!(r##"
[ORIGINAL DOCUMENT]:
{}

[NETWORKS]:
{}
"##, original_document, all_network_jsons);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       INFER REDUNDANT NETWORKS - LLM REQUEST                  ║");
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

        Self::send_redundant_networks_request(system_prompt, &user_prompt).await
    }

    async fn send_redundant_networks_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(RedundantNetworksResponse, RedundantNetworksResponseMetadata), Errors> {
        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "redundant_networks",
            true,
            json!({
                "type": "object",
                "properties": {
                    "canonical": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "IDs of networks that are kept as distinct resources"
                    },
                    "eliminated": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "ID of the eliminated network"
                                },
                                "maps_to": {
                                    "type": "string",
                                    "description": "ID of the canonical network this maps to"
                                },
                                "reason": {
                                    "type": "string",
                                    "description": "One sentence explaining why this network was eliminated"
                                }
                            },
                            "required": ["id", "maps_to", "reason"],
                            "additionalProperties": false
                        },
                        "description": "Networks that were eliminated and the canonical network they map to"
                    }
                },
                "required": ["canonical", "eliminated"],
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

                    let redundancy_response = {
                        match serde_json::from_str::<RedundantNetworksResponse>(content) {
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
                            RedundantNetworksResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            RedundantNetworksResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok((redundancy_response, metadata))
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
