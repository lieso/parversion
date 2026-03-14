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
pub struct FieldInferenceResponse {
    pub field_name: String,
    pub description: String,
    pub data_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldInferenceResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTransformationResponse {
    pub data: Option<FieldInferenceResponse>,
    pub metadata: NodeTransformationResponseMetadata,
}

pub struct NodeAnalysis;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponseMetadata {
    tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponse {
    is_boilerplate: bool,
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
                field,
                field_snippets.clone(),
                document_summary,
            ).await?,
        };

        tokens += metadata.tokens;

        if should_eliminate_response.is_boilerplate {
            let result = NodeTransformationResponse {
                data: None,
                metadata: NodeTransformationResponseMetadata {
                    tokens,
                }
            };

            return Ok(result);
        }

        let (field_inference_response, metadata): (FieldInferenceResponse, FieldInferenceResponseMetadata) = match field {
            "text" => Self::infer_text_data_field(
                value,
                field_snippets.clone(),
                document_summary,
            ).await?,
            _ => Self::infer_attribute_data_field(
                field,
                field_snippets.clone(),
                document_summary,
            ).await?,
        };

        tokens += metadata.tokens;

        let result = NodeTransformationResponse {
            data: Some(field_inference_response),
            metadata: NodeTransformationResponseMetadata {
                tokens,
            }
        };

        Ok(result)
    }

    async fn infer_text_data_field(
        value: &str,
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<(FieldInferenceResponse, FieldInferenceResponseMetadata), Errors> {
        log::trace!("In infer_text_data_field");

        let system_prompt = format!(r##"
You are an expert data engineer reverse-engineering a backend data model from rendered HTML.

### Document Context:
Website Summary: {}

### Goal:
Analyze the target text node (delimited by <!-- Target node: Start -->) and infer the original data field it represents.

### Instructions:
1. **Field Name**: Create a semantically accurate `snake_case` variable name. It should reflect the data's role within the context of the website.
2. **Description**: Write a concise description of what this data represents, as if writing documentation for an API.
3. **Data Type**: Identify the likely primitive type (string, number, boolean, url, datetime).

### Response Format:
Respond with valid JSON:
{{
  "field_name": "string",
  "description": "string",
  "data_type": "string"
}}
"##, document_summary);

        let examples = snippets
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (index, snippet)| {
                acc.push_str(&format!(
                    r##"
Example {}:
{}
"##,
                    index + 1,
                    snippet
                ));
                acc
            });

        let user_prompt = format!(r##"
[text]
{}

[Examples]
{}
"##, value.trim(), examples);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       INFER TEXT DATA FIELD - LLM REQUEST                     ║");
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

        Self::send_field_inference_request(&system_prompt, &user_prompt).await
    }

    async fn infer_attribute_data_field(
        field: &str,
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<(FieldInferenceResponse, FieldInferenceResponseMetadata), Errors> {
        log::trace!("In infer_attribute_data_field");

        let system_prompt = format!(r##"
You are an expert data engineer reverse-engineering a backend data model from rendered HTML.

### Document Context:
Website Summary: {}

### Goal:
Analyze the target HTML attribute (delimited by <!-- Target node: Start -->) and infer the original data field it represents.

### Instructions:
1. **Field Name**: Create a semantically accurate `snake_case` variable name. It should reflect the data's role within the context of the website.
2. **Description**: Write a concise description of what this data represents, as if writing documentation for an API.
3. **Data Type**: Identify the likely primitive type (string, number, boolean, url, datetime).

### Response Format:
Respond with valid JSON:
{{
  "field_name": "string",
  "description": "string",
  "data_type": "string"
}}
"##, document_summary);

        let examples = snippets
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (index, snippet)| {
                acc.push_str(&format!(
                    r##"
Example {}:
{}
"##,
                    index + 1,
                    snippet
                ));
                acc
            });

        let user_prompt = format!(r##"
[attribute]
{}

[Examples]
{}
"##, field.trim(), examples);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       INFER ATTRIBUTE DATA FIELD - LLM REQUEST                ║");
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

        Self::send_field_inference_request(&system_prompt, &user_prompt).await
    }

    async fn send_field_inference_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(FieldInferenceResponse, FieldInferenceResponseMetadata), Errors> {
        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "field_inference",
            true,
            json!({
                "type": "object",
                "properties": {
                    "field_name": {
                        "type": "string",
                        "description": "The inferred snake_case variable name"
                    },
                    "description": {
                        "type": "string",
                        "description": "Concise description of the data"
                    },
                    "data_type": {
                        "type": "string",
                        "description": "The likely primitive type (string, number, boolean, url, datetime)"
                    }
                },
                "required": ["field_name", "description", "data_type"],
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
                        match serde_json::from_str::<FieldInferenceResponse>(content) {
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
                            FieldInferenceResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            FieldInferenceResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok((inference_response, metadata))
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

    async fn should_eliminate_text(
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<(EliminationResponse, EliminationResponseMetadata), Errors> {
        log::trace!("In should_eliminate_text");

        let system_prompt = format!(r##"
You are an expert data engineer specializing in reverse-engineering data models from rendered HTML. 

Your goal is to determine if a specific text node represents "Application Data" (dynamic content that would be stored in a database/API) or "UI Boilerplate" (static text hardcoded into the frontend template).

### Document Context:
The following is a summary of the website being analyzed:
{}

### Target Node Identification:
You will be provided with one or more HTML snippets containing the text node to analyze. 
To provide crucial context, surrounding HTML is included. The specific text node you must evaluate is explicitly delimited with HTML comments like so:
<!-- Target node: Start -->Text node content here<!-- Target node: End -->

### Classification Criteria:
You must flag the target text node as "Boilerplate" (true) if it matches any of the following:
1. **Structural/Presentational Symbols**: Pipes (|), bullets (•), arrows (→), or characters used solely for visual layout.
2. **Static UI Affordances**: Labels like "Search", "Add to Cart", "Submit", "Related Posts", or "Follow us on Twitter".
3. **Hardcoded Metadata**: Copyright notices, "All rights reserved", or version numbers.
4. **Advertisements**: Promotional text or sponsored content that is ancillary to the page's core data model.
5. **Empty/Placeholder Text**: Text used as a layout filler that contains no real information.

Respond that it is NOT "Boilerplate" (false) if:
- The text is dynamic content (e.g., a product name, a user's comment, a news headline, a price, or a date).
- The text represents the primary information a user came to this specific page to consume.

### Response Format:
You must respond with valid JSON containing exactly one field:
{{
  "is_boilerplate": boolean
}}
"##, document_summary);

        let examples = snippets
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (index, snippet)| {
                acc.push_str(&format!(
                    r##"
Example {}:
{}
"##,
                    index + 1,
                    snippet
                ));
                acc
            });

        let user_prompt = format!(r##"
[Examples]
{}
"##, examples);

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
            "is_boilerplate_text",
            true,
            json!({
                "type": "object",
                "properties": {
                    "is_boilerplate": {
                        "type": "boolean",
                        "description": "Is the text node boilerplate"
                    }
                },
                "required": ["is_boilerplate"],
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
        field: &str,
        snippets: Vec<&str>,
        document_summary: &str
    ) -> Result<(EliminationResponse, EliminationResponseMetadata), Errors> {
        log::trace!("In should_eliminate_attribute");

        let system_prompt = format!(r##"
You are an expert data engineer specializing in reverse-engineering data models from rendered HTML. 

Your goal is to determine if a specific HTML attribute represents "Application Data" (dynamic content that would be stored in a database/API) or "UI Boilerplate" (static text hardcoded into the frontend template, code, or ancillary content).

### Document Context:
The following is a summary of the website being analyzed:
{}

### Target Attribute Identification:
You will be provided with one or more HTML snippets containing the attribute to analyze. 
To provide crucial context, surrounding HTML is included. The specific node containing the attribute you must evaluate is explicitly delimited with HTML comments like so:
<!-- Target node: Start --><a href="https://example.com" other-attribute="val"><!-- Target node: End -->

### Classification Criteria:
You must flag the target attribute as "Boilerplate" (true) if it matches any of the following:
1. **Advertisements**: Promotional URLs or text, sponsored content, or tracking codes that are ancillary to the page's core data model.
2. **Code or State**: The attribute value contains serialized code, JSON, configuration state, or encoded data meant for the browser/frontend, rather than human-readable natural language.
3. **Static UI/Styling**: Classes, IDs, inline styles, ARIA labels, or layout-specific attributes that are hardcoded and not dynamic data.
4. **Hardcoded URLs**: Links to static assets, internal navigational pages, or generic social media profiles (unless the core purpose is listing such profiles).

Respond that it is NOT "Boilerplate" (false) if:
- The attribute contains dynamic content (e.g., a specific article URL, a user profile link, an image source for a product, etc.).
- A user would intentionally read or interact with the attribute's value (e.g., href, src) as part of their core purpose in visiting the website.

### Response Format:
You must respond with valid JSON containing exactly one field:
{{
  "is_boilerplate": boolean
}}
"##, document_summary);

        let examples = snippets
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (index, snippet)| {
                acc.push_str(&format!(
                    r##"
Example {}:
{}
"##,
                    index + 1,
                    snippet
                ));
                acc
            });

        let user_prompt = format!(r##"
[Attribute]
{}

[Examples]
{}
"##, field.trim(), examples);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       SHOULD ELIMINATE ATTRIBUTE - LLM REQUEST                ║");
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
            "is_boilerplate_attribute",
            true,
            json!({
                "type": "object",
                "properties": {
                    "is_boilerplate": {
                        "type": "boolean",
                        "description": "Is the attribute boilerplate"
                    }
                },
                "required": ["is_boilerplate"],
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
                                    "║          SHOULD ELIMINATE ATTRIBUTE- REQUEST COMPLETE         ║"
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

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
