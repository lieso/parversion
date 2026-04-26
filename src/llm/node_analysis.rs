use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

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
pub enum LineageClassification {
    Acyclic,
    Uniform,
    Diverging(Vec<String>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeGroupsData {
    pub groups: HashMap<String, LineageClassification>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeGroupsResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct NodeGroupLLMEntry {
    lineage: String,
    classification: String,
    indexed_lineages: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct NodeGroupsLLMResponse {
    groups: Vec<NodeGroupLLMEntry>,
}

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
        field_snippets: Vec<String>,
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
        snippets: Vec<String>,
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
        snippets: Vec<String>,
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
        snippets: Vec<String>,
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
        snippets: Vec<String>,
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

    pub async fn get_node_groups(
        user_prompt: String,
    ) -> Result<(
        NodeGroupsData,
        NodeGroupsResponseMetadata,
    ), Errors> {
        log::trace!("In get_node_groups");

        let system_prompt = r##"
You are an expert data engineer analyzing the structural and semantic content of HTML document nodes to determine how they should be grouped for data extraction into a JSON schema.

### Context:
You will be given an acyclic lineage identifier followed by one or more lineages. Each lineage represents a distinct structural path through an HTML document. For each lineage you will see the HTML content of nodes that share that path. Some nodes may also have diverging indexed lineages — structural variants that distinguish subsets of nodes within the same lineage.

An indexed lineage is a variant of a node's lineage computed by injecting the positional index of one of its ancestors (its position among siblings at that level) into the lineage hash. A full set of indexed lineages is computed for each node, one per ancestor level. The diverging indexed lineages shown are those that are not common across all nodes under the same lineage — meaning they reveal positional differences between nodes that share a structural path but sit at different positions in the document tree.

**Important**: The presence of diverging indexed lineages does NOT by itself mean the content is semantically non-uniform. Nodes in a list (e.g. article URLs, product prices, user comments) will each have a unique positional indexed lineage simply because they occupy different positions — but they are all the same kind of data. Only use diverging indexed lineages as discriminators when different subsets of the content are genuinely semantically different kinds of data (e.g. some nodes are action links while others are counts, or some are headings while others are body text — those are different things even though they share the same structural path).

### Your Task:
For each lineage, determine the appropriate classification by looking at the actual content of the nodes, not just whether diverging indexed lineages are present:

1. **Acyclic**: All lineages under this acyclic lineage represent semantically uniform content — the same kind of data that could be given a single field name, description, and data type in a JSON schema. Use this when every node across every lineage represents the same thing (e.g. they are all article URLs, or all comment bodies).

2. **Uniform**: This lineage is semantically distinct from the other lineages, but all nodes within this lineage represent the same kind of data. Use this when lineages represent different things from each other, but within this particular lineage the content is consistent. This is the correct classification when all nodes under a lineage are the same type of content (all URLs, all titles, all prices) even if each has a unique diverging indexed lineage due to list position.

3. **Diverging**: The nodes within this lineage contain genuinely different kinds of data — some nodes represent one thing and other nodes represent a completely different thing, and the diverging indexed lineages are what distinguish these two groups. Use this only when you can clearly identify two or more semantically distinct content types among the nodes (e.g. a mix of action links and numeric counts, or a mix of headings and body text). Do NOT use this just because each node has a unique positional indexed lineage.

### Rules:
- Every lineage present in the input must appear in your response exactly once.
- If you choose Acyclic for one lineage, you should choose Acyclic for all lineages (they all share the same acyclic lineage).
- Set `indexed_lineages` to null for Acyclic and Uniform classifications.
- For Diverging, `indexed_lineages` must contain only the small number of indexed lineage strings that discriminate the distinct content types — not one per context.

### Response Format:
Respond with valid JSON:
{
  "groups": [
    {
      "lineage": "<lineage string>",
      "classification": "Acyclic" | "Uniform" | "Diverging",
      "indexed_lineages": null | ["<indexed lineage string>", ...]
    }
  ]
}
"##;

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║               GET NODE GROUPS - LLM REQUEST                  ║");
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
            "node_groups",
            true,
            json!({
                "type": "object",
                "properties": {
                    "groups": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "lineage": {
                                    "type": "string",
                                    "description": "The lineage string being classified"
                                },
                                "classification": {
                                    "type": "string",
                                    "enum": ["Acyclic", "Uniform", "Diverging"],
                                    "description": "The grouping classification for this lineage"
                                },
                                "indexed_lineages": {
                                    "type": ["array", "null"],
                                    "items": { "type": "string" },
                                    "description": "Diverging indexed lineage strings that define subgroups, or null"
                                }
                            },
                            "required": ["lineage", "classification", "indexed_lineages"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["groups"],
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
                    log::debug!("┌─── LLM RESPONSE CONTENT ──────────────────────────────────────┐");
                    log::debug!("{}", content);
                    log::debug!("└───────────────────────────────────────────────────────────────┘");
                    log::debug!("");

                    let llm_response = match serde_json::from_str::<NodeGroupsLLMResponse>(content) {
                        Ok(parsed) => {
                            log::debug!("┌─── PARSED RESPONSE ───────────────────────────────────────────┐");
                            log::debug!("{:?}", parsed);
                            log::debug!("└───────────────────────────────────────────────────────────────┘");
                            log::debug!("");
                            parsed
                        }
                        Err(e) => {
                            log::error!("╔═══════════════════════════════════════════════════════════════╗");
                            log::error!("║                    PARSE ERROR                                ║");
                            log::error!("╚═══════════════════════════════════════════════════════════════╝");
                            log::error!("Failed to parse LLM response: {}", e);
                            return Err(Errors::UnexpectedError);
                        }
                    };

                    let groups = llm_response.groups.into_iter().filter_map(|entry| {
                        let classification = match entry.classification.as_str() {
                            "Acyclic" => Some(LineageClassification::Acyclic),
                            "Uniform" => Some(LineageClassification::Uniform),
                            "Diverging" => entry.indexed_lineages.map(LineageClassification::Diverging),
                            other => {
                                log::error!("Unrecognised classification from LLM: {}", other);
                                None
                            }
                        };
                        classification.map(|c| (entry.lineage, c))
                    }).collect();

                    let metadata = if let Some(usage) = response.usage {
                        NodeGroupsResponseMetadata { tokens: usage.total_tokens as u64 }
                    } else {
                        NodeGroupsResponseMetadata { tokens: 0 }
                    };

                    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
                    log::debug!("║                                                               ║");
                    log::debug!("║             GET NODE GROUPS - REQUEST COMPLETE                ║");
                    log::debug!("║                                                               ║");
                    log::debug!("╚═══════════════════════════════════════════════════════════════╝");

                    Ok((NodeGroupsData { groups }, metadata))
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

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
