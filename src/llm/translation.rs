use std::collections::HashMap;
use crate::prelude::*;
use crate::environment::get_env_variable;
use crate::path::Path;
use crate::path_segment::PathSegmentKind;
use openrouter_rs::{OpenRouterClient, api::chat::*, types::{Role, ResponseFormat}};
use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MatchTargetSchemaResponse {
    pub is_incompatible: bool,
    pub json_path: Option<String>,
}

pub struct Translation;

impl Translation {
    pub async fn match_target_schema(
        marked_schema_node: &String,
        target_schema: &String
    ) -> Result<MatchTargetSchemaResponse, Errors> {
        log::trace!("In match_target_schema");

        let system_prompt = r##"
Your task is to compare two JSON schemas and attempt to match a target schema field from the first with the second, if there is an appropriate equivalent.

Both schemas are expected to represent the same type of resource, but may have different key structures and naming conventions. However, the schemas may be incompatible - meaning they represent fundamentally different resources or the target field has no reasonable equivalent in the second schema.

The first JSON schema will be an incomplete snippet, and the schema field to match against will be found inside delimiter strings:
START TARGET SCHEMA KEY >>>
<<< END TARGET SCHEMA KEY

Please provide the following information:

1. (is_incompatible): Set to true if the schemas are incompatible or if there is no appropriate equivalent field in the second schema. Set to false if a match can be made.

2. (json_path): A JSON path pointing to the field in the second schema that is equivalent to the target schema field from the first schema. This path should be relative to the JSON schema itself, not the resulting JSON document. Set to null if is_incompatible is true or if no match can be found.

For example, if the target field matches a field called "issueDate" nested under "properties" in the second schema, the json_path would be: '$.properties.issueDate'
"##;

        let user_prompt = format!(r##"
[FIRST JSON SCHEMA]:
{}

[SECOND JSON SCHEMA]:
{}
"##, marked_schema_node, target_schema);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║              MATCH TARGET SCHEMA - LLM REQUEST                ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");
        log::debug!("┌─── MARKED SCHEMA NODE ────────────────────────────────────────┐");
        log::debug!("{}", marked_schema_node);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");
        log::debug!("┌─── TARGET SCHEMA ─────────────────────────────────────────────┐");
        log::debug!("{}", target_schema);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
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
            "match_target_schema",
            true,
            json!({
                "type": "object",
                "properties": {
                    "is_incompatible": {
                        "type": "boolean",
                        "description": "Whether the schemas are incompatible or no appropriate match exists"
                    },
                    "json_path": {
                        "type": ["string", "null"],
                        "description": "JSON path to the matching field in the target schema, or null if no match"
                    }
                },
                "required": ["is_incompatible", "json_path"],
                "additionalProperties": false
            })
        );

        let request = ChatCompletionRequest::builder()
            .model("google/gemini-3-pro-preview")
            .messages(vec![
                Message::new(Role::System, system_prompt),
                Message::new(Role::User, user_prompt)
            ])
            .response_format(response_format)
            .build()
            .expect("could not create request");

        match client.send_chat_completion(&request).await {
            Ok(response) => {
                log::debug!("┌─── RAW LLM RESPONSE ──────────────────────────────────────────┐");
                log::debug!("{:?}", response);
                log::debug!("└───────────────────────────────────────────────────────────────┘");
                log::debug!("");

                if let Some(content) = response.choices[0].content() {
                    log::debug!("┌─── LLM RESPONSE CONTENT ──────────────────────────────────────┐");
                    log::debug!("{}", content);
                    log::debug!("└───────────────────────────────────────────────────────────────┘");
                    log::debug!("");

                    match serde_json::from_str::<MatchTargetSchemaResponse>(content) {
                        Ok(parsed_response) => {
                            log::debug!("┌─── PARSED RESPONSE ───────────────────────────────────────────┐");
                            log::debug!("{:?}", parsed_response);
                            log::debug!("└───────────────────────────────────────────────────────────────┘");
                            log::debug!("");
                            log::debug!("╔═══════════════════════════════════════════════════════════════╗");
                            log::debug!("║                                                               ║");
                            log::debug!("║            MATCH TARGET SCHEMA - REQUEST COMPLETE             ║");
                            log::debug!("║                                                               ║");
                            log::debug!("╚═══════════════════════════════════════════════════════════════╝");

                            Ok(parsed_response)
                        }
                        Err(e) => {
                            log::error!("╔═══════════════════════════════════════════════════════════════╗");
                            log::error!("║                    PARSE ERROR                                ║");
                            log::error!("╚═══════════════════════════════════════════════════════════════╝");
                            log::error!("Failed to parse LLM response: {}", e);
                            Err(Errors::UnexpectedError)
                        }
                    }
                } else {
                    log::error!("╔═══════════════════════════════════════════════════════════════╗");
                    log::error!("║                    NO CONTENT ERROR                           ║");
                    log::error!("╚═══════════════════════════════════════════════════════════════╝");
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

    pub async fn match_path_variables(
        schema_node_path: &Path,
        target_node_path: &Path,
        snippet: &String,
        target_schema: &String,
    ) -> Result<HashMap<char, PathSegmentKind>, Errors> {
        log::trace!("In match_path_variables");

        unimplemented!()
    }

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
