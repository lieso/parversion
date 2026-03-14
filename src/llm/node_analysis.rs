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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponseMetadata {
    tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponse {
    should_eliminate: bool,
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
        
        let (should_eliminate_response, metadata): (EliminationResponse, EliminationResponseMetadata) = match field {
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
            tokens += metadata.tokens;

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
    ) -> Result<(EliminationResponse, EliminationResponseMetadata), Errors> {
        log::trace!("In should_eliminate_text");

        let system_prompt = r##"

        "##;

        let user_prompt = format!(r##"

        "##);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║         SHOULD ELIMINATE TEXT - LLM REQUEST                   ║");
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
            "is_unmeaningful_text",
            true,
            json!({
                "type": "object",
                "properties": {
                    "is_unmeaningful": {
                        "type": "boolean",
                        "description": "Is the text node unmeaningful"
                    }
                },
                "required": ["is_unmeaningful"],
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

                    let elimination_response = {
                        match serde_json::from_str::<EliminationResponse>(content) {
                            Ok(parsed_response) => {
                                log::debug!(
                                    "┌─── PARSED RESPONSE ───────────────────────────────────────────┐"
                                );
                                log::debug!("{:?}", parsed_response);
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
                                    "║            SHOULD ELIMINATE TEXT- REQUEST COMPLETE            ║"
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
                            EliminationResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            EliminationResponseMetadata {
                                tokens: 0,
                            }
                        }
                    };

                    Ok((elimination_response, metadata))
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

    async fn should_eliminate_attribute(
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<(EliminationResponse, EliminationResponseMetadata), Errors> {
        log::trace!("In should_eliminate_attribute");

        unimplemented!();
    }

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
