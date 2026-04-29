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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationshipItem {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub relationship_type: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationshipsResponse {
    pub relationships: Vec<NetworkRelationshipItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationshipsResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ValueXPath {
    pub name: String,
    pub xpath: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParentChildLinkResponse {
    pub candidate_xpath: String,
    pub parent_value_xpaths: Vec<ValueXPath>,
    pub candidate_value_xpaths: Vec<ValueXPath>,
    pub filter_function: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParentChildLinkResponseMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompositionLinkResponse {
    pub forward_xpath: String,
    pub reverse_xpath: String,
    pub merged_variable_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompositionLinkResponseMetadata {
    pub tokens: u64,
}

pub struct NetworkRelationships;

impl NetworkRelationships {
    pub async fn get_parent_child_link(
        snippet: &str,
    ) -> Result<(ParentChildLinkResponse, ParentChildLinkResponseMetadata), Errors> {
        log::trace!("In get_parent_child_link");

        let system_prompt = r##"
You are given an HTML document containing multiple instances of the same network. Each instance is marked with comments:

<!-- Target Network: Start -->
<element ...>
<!-- Target Network: End -->

The opening tag immediately following Start is the anchor element for that instance.

The instances have a parent-child relationship: some instances are hierarchical children of other instances of the same network. Your task is to describe how to confirm, at runtime, whether one instance is a direct child of another.

Provide the following:

1. candidate_xpath — An XPath expression relative to one network anchor that navigates to one or more neighboring network anchors of the same type. These are candidates that may or may not be direct children.

2. parent_value_xpaths — An array of { name, xpath } objects. Each xpath is relative to the parent anchor and extracts a specific text or attribute value that will be used to confirm the relationship. Use short snake_case names.

3. candidate_value_xpaths — An array of { name, xpath } objects. Each xpath is relative to the candidate anchor and extracts a corresponding value.

4. filter_function — The body of a JavaScript function that returns true if the candidate is a direct child of the parent, and false otherwise. The function has access to variables named parent_{name} and candidate_{name}, corresponding to the values extracted by parent_value_xpaths and candidate_value_xpaths respectively. All values are strings as extracted from the DOM. Do not include a function declaration — provide only the body.

Base your XPaths and filter logic strictly on structure and values visible in the provided HTML. Do not infer relationships based on assumptions about the document type or domain.

Prefer simple XPath syntax. Avoid verbose defensive patterns where a simpler equivalent is reliable given the document structure.

Output
Respond with valid JSON in the following format:
{
  "candidate_xpath": "XPath from one network anchor to neighboring same-type anchors",
  "parent_value_xpaths": [
    { "name": "snake_case_name", "xpath": "XPath relative to parent anchor" }
  ],
  "candidate_value_xpaths": [
    { "name": "snake_case_name", "xpath": "XPath relative to candidate anchor" }
  ],
  "filter_function": "return candidate_level === String(Number(parent_level) + 1);"
}

Do not include any explanation outside the JSON.
"##;

        let user_prompt = format!(r##"
[DOCUMENT SNIPPET]:
{}
"##, snippet);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       PARENT CHILD LINK - LLM REQUEST                         ║");
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

        Self::send_parent_child_link_request(system_prompt, &user_prompt).await
    }

    async fn send_parent_child_link_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(ParentChildLinkResponse, ParentChildLinkResponseMetadata), Errors> {
        let client = Self::build_client();

        let value_xpath_schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "snake_case name used as a variable in the filter function"
                },
                "xpath": {
                    "type": "string",
                    "description": "XPath relative to the network anchor that extracts a value"
                }
            },
            "required": ["name", "xpath"],
            "additionalProperties": false
        });

        let response_format = ResponseFormat::json_schema(
            "parent_child_link",
            true,
            json!({
                "type": "object",
                "properties": {
                    "candidate_xpath": {
                        "type": "string",
                        "description": "XPath from one network anchor to neighboring same-type candidate anchors"
                    },
                    "parent_value_xpaths": {
                        "type": "array",
                        "items": value_xpath_schema.clone(),
                        "description": "XPaths relative to the parent anchor that extract values for the filter"
                    },
                    "candidate_value_xpaths": {
                        "type": "array",
                        "items": value_xpath_schema,
                        "description": "XPaths relative to the candidate anchor that extract values for the filter"
                    },
                    "filter_function": {
                        "type": "string",
                        "description": "JavaScript function body returning true if the candidate is a direct child of the parent"
                    }
                },
                "required": ["candidate_xpath", "parent_value_xpaths", "candidate_value_xpaths", "filter_function"],
                "additionalProperties": false
            }),
        );

        let request = ChatCompletionRequest::builder()
            .model("gpt-5")
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

                    let parent_child_link_response = {
                        match serde_json::from_str::<ParentChildLinkResponse>(content) {
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
                            ParentChildLinkResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            ParentChildLinkResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok((parent_child_link_response, metadata))
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

    pub async fn get_composition_link(
        snippet: &str,
    ) -> Result<(CompositionLinkResponse, CompositionLinkResponseMetadata), Errors> {
        log::trace!("In get_composition_link");

        let system_prompt = r##"
You are given an HTML document containing instances of two networks that have a composition relationship. This means each instance of Network A and one corresponding instance of Network B together form a single complete resource. They are separate elements in the DOM — neither is nested inside the other.

Each network instance has an anchor element marked with comments:

<!-- Target Network A: Start -->
<element ...>
<!-- Target Network A: End -->

The opening tag immediately following Start is the anchor element.

Your task is to:

1. Provide two XPath expressions, each relative to an anchor element as the context node:
   - forward_xpath — evaluated from a Network A anchor, must select exactly one Network B anchor
   - reverse_xpath — evaluated from a Network B anchor, must select exactly one Network A anchor

   Both XPaths must be relative (do not start with /). Each must reliably select exactly one element across all instances shown in the document. If a candidate XPath would select more than one element for any instance shown, it is incorrect.

   Base your XPaths strictly on the structure visible in the provided HTML. Do not infer paths that are not evidenced by the examples.

2. Provide a semantically accurate `snake_case` variable name (merged_variable_name) that would be appropriate for naming the combined resource that results from merging Network A and Network B into a single flat JSON object.

Output
Respond with valid JSON in the following format:
{
  "forward_xpath": "XPath from Network A anchor to Network B anchor",
  "reverse_xpath": "XPath from Network B anchor to Network A anchor",
  "merged_variable_name": "snake_case name for the merged resource"
}

Do not include any explanation outside the JSON.
"##;

        let user_prompt = format!(r##"
[DOCUMENT SNIPPET]:
{}
"##, snippet);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       COMPOSITION LINK - LLM REQUEST                          ║");
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

        Self::send_composition_link_request(system_prompt, &user_prompt).await
    }

    async fn send_composition_link_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(CompositionLinkResponse, CompositionLinkResponseMetadata), Errors> {
        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "composition_link",
            true,
            json!({
                "type": "object",
                "properties": {
                    "forward_xpath": {
                        "type": "string",
                        "description": "XPath from Network A anchor to Network B anchor"
                    },
                    "reverse_xpath": {
                        "type": "string",
                        "description": "XPath from Network B anchor to Network A anchor"
                    },
                    "merged_variable_name": {
                        "type": "string",
                        "description": "A semantically accurate snake_case name for the merged resource"
                    }
                },
                "required": ["forward_xpath", "reverse_xpath", "merged_variable_name"],
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

                    let composition_link_response = {
                        match serde_json::from_str::<CompositionLinkResponse>(content) {
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
                            CompositionLinkResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            CompositionLinkResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok((composition_link_response, metadata))
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

    pub async fn identify_relationships(
        original_document: &str,
        all_network_jsons: &str,
    ) -> Result<(NetworkRelationshipsResponse, NetworkRelationshipsResponseMetadata), Errors> {
        log::trace!("In identify_relationships");

        let system_prompt = r##"
You are given a set of canonical JSON networks extracted from an HTML document. Each network has been deduplicated and represents a distinct resource. Your task is to identify the
relationships between these networks.

For each relationship you identify, assign one of the following types:

composition — the two networks are separate, non-nested fragments of the same resource in the DOM and should be merged into a single flat object. from is the primary network, to is the supplementary one. Do not classify as composition if to's structure already appears embedded as a sub-object within from's examples — that is a nesting relationship, not composition.

parent_child — instances of from can be children of other instances of from. Set from and to to the same network ID.

A network may have more than one relationship with another network, including with itself. Identify and list all relationships that are evidenced, not just the most prominent one.

Only include relationships that are directly evidenced by the network examples or the original document. Do not infer relationships based on assumptions about the document type or
domain.

Output
Respond with valid JSON in the following format:
{
  "relationships": [
    {
      "from": "network_id",
      "to": "network_id",
      "type": "composition|parent_child",
      "reason": "one sentence"
    }
  ]
}

Do not include any explanation outside the JSON.
"##;
        let user_prompt = format!(r##"
[ORIGINAL DOCUMENT]:
{}

[NETWORKS]:
{}
"##, original_document, all_network_jsons);

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║       IDENTIFY RELATIONSHIPS - LLM REQUEST                    ║");
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

        Self::send_relationships_request(system_prompt, &user_prompt).await
    }

    async fn send_relationships_request(
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(NetworkRelationshipsResponse, NetworkRelationshipsResponseMetadata), Errors> {
        let client = Self::build_client();

        let response_format = ResponseFormat::json_schema(
            "network_relationships",
            true,
            json!({
                "type": "object",
                "properties": {
                    "relationships": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": {
                                    "type": "string",
                                    "description": "ID of the source network"
                                },
                                "to": {
                                    "type": "string",
                                    "description": "ID of the target network"
                                },
                                "type": {
                                    "type": "string",
                                    "enum": ["composition", "parent_child"],
                                    "description": "The type of relationship between the two networks"
                                },
                                "reason": {
                                    "type": "string",
                                    "description": "One sentence explaining the relationship"
                                }
                            },
                            "required": ["from", "to", "type", "reason"],
                            "additionalProperties": false
                        },
                        "description": "Identified relationships between networks"
                    }
                },
                "required": ["relationships"],
                "additionalProperties": false
            }),
        );

        let request = ChatCompletionRequest::builder()
            .model("gpt-5")
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

                    let relationships_response = {
                        match serde_json::from_str::<NetworkRelationshipsResponse>(content) {
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
                            NetworkRelationshipsResponseMetadata {
                                tokens: usage.total_tokens.clone() as u64,
                            }
                        } else {
                            NetworkRelationshipsResponseMetadata { tokens: 0 }
                        }
                    };

                    Ok((relationships_response, metadata))
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

    pub async fn get_canonical_networks(
        original_document: &str,
        all_network_jsons: &str,
    ) -> Result<(RedundantNetworksResponse, RedundantNetworksResponseMetadata), Errors> {
        log::trace!("In get_canonical_networks");

        let system_prompt = r##"
You are given a set of JSON networks extracted from an HTML document. Each network has a unique ID and one or more examples showing the JSON keys and values that were observed together in the DOM.

Your task is to deduplicate this set of networks so that each remaining network represents a distinct resource. Perform the following two operations:

1. Remove nested networks
A nested network is one whose structure appears as a non-repeating subtree within a single instance of another network. If network A's structure appears embedded inside network B as a
fixed sub-object, remove A and map it to B.

Pay particular attention to small networks consisting of only two or three keys that appear as a named sub-object within a larger network's examples. These are strong candidates for elimination regardless of how semantically self-contained they appear.

Exception: If network A appears as an element within an array inside network B, do not eliminate A. A repeated item within a collection is a distinct resource, not a nested subtree.
Only eliminate a network if it appears as a non-repeating embedded object within a single instance of another network.

2. Remove duplicate networks
A duplicate network represents the same resource as another network but differs in completeness, key naming variation, or number of examples. When two networks represent the same
resource, keep one and eliminate the other.

When deciding which network to keep, apply these criteria in order:
- Prefer the network with the most consistent structure across its examples
- Prefer the network that has more examples
- Prefer the flatter structure when nesting adds no semantic value
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
