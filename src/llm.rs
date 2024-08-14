use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use std::env;

use crate::node_data_structure::{NodeDataStructure};
use crate::node_data::{NodeData, ElementData, TextData};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMDataStructureResponse {
    #[serde(deserialize_with = "empty_string_as_none")]
    root_node_xpath: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    parent_node_xpath: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    next_item_xpath: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMElementDataResponse {
    attribute: String,
    name: String,
    is_page_link: Option<bool>,
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> NodeDataStructure {
    log::trace!("In interpret_data_structure");

    assert!(snippets.len() > 0, "Did not receive any snippets");

    let examples = snippets.iter().enumerate().fold(
        String::new(),
        |mut acc, (index, snippet)| {
            acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
            acc
        }
    );

    let system_prompt = format!(r##"
Your task is to infer implicit relationships for HTML element nodes of a particular type. At least one example of the element node will be provided along with some surrounding HTML providing necessary context. The target node to be analyzed will be delimited with an HTML comment.

An example of an implicit relationship is when a list of element nodes representing weather forecasts get rendered where each HTML element node represents a forecast for a particular day and a distinct node is used for the following day. Another example is when a discussion forum website renders a list of element nodes where each node implicitly represents a user reply; in this case there is a recursive relationship between these element nodes with each item either being a root node or having a parent relationship to another node.

Determine if any of the following relationships apply to the element node I will provide you. It's possible for multiple relationships to apply to a single element and you should anticipate the possibility that none may apply to the node:

1. Does the element represent a recursive relationship to other elements? If so, please provide the following:
   • root_node_xpath: Provide a generic XPath expression that would test if elements of this type are root nodes. 
   • parent_node_xpath: Provide generic XPath expression that would select the element node's parent if it is not a root node.
2. Does the element represent an item in a meaningful list? If so, please provide the following:
   • next_item_xpath: Provide generic XPath expression that would select the next item in the list.
"##);
    let user_prompt = format!(r##"
Example(s) of the node to be analyzed:

---

{}

---
"##, examples);
    log::debug!("user_prompt: {}", user_prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model": "gpt-4o-2024-08-06",
        "temperature": 0,
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": user_prompt
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "relationship_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "root_node_xpath": {
                            "type": "string"
                        },
                        "parent_node_xpath": {
                            "type": "string"
                        },
                        "next_item_xpath": {
                            "type": "string"
                        }
                    },
                    "required": ["root_node_xpath", "parent_node_xpath", "next_item_xpath"],
                    "additionalProperties": false
                }
            }
        }
    });
    let url = "https://api.openai.com/v1/chat/completions";
    let authorization = format!("Bearer {}", openai_api_key);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&request_json)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, authorization)
        .send()
        .await;
    let response = response.expect("Failed to send request to OpenAI");

    let json_response = response.json::<serde_json::Value>().await.expect("Unable to get JSON from response");
    log::debug!("json_response: {:?}", json_response);
    let json_response = json_response["choices"].as_array().unwrap();
    let json_response = &json_response[0]["message"]["content"].as_str().unwrap();

    let llm_data_structure_response = serde_json::from_str::<LLMDataStructureResponse>(json_response)
        .expect("Could not parse json response as LLMDataStructureResponse");
    log::debug!("llm_data_structure_response: {:?}", llm_data_structure_response);

    NodeDataStructure {
        root_node_xpath: llm_data_structure_response.root_node_xpath,
        parent_node_xpath: llm_data_structure_response.parent_node_xpath,
        next_item_xpath: llm_data_structure_response.next_item_xpath,
   }
}

pub async fn interpret_element_data(meaningful_attributes: Vec<String>, snippets: Vec<String>) -> Vec<NodeData> {
    log::trace!("In interpret_element_data");

    assert!(snippets.len() > 0, "Did not receive any snippets");

    let examples = snippets.iter().enumerate().fold(
        String::new(),
        |mut acc, (index, snippet)| {
            acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
            acc
        }
    );
    let attributes = meaningful_attributes.iter().fold(
        String::new(),
        |acc, attr| {
            format!("{}\n{}", acc, attr)
        }
    );

    let system_prompt = format!(r##"
Your task is to interpret the meaning of HTML element attributes, provide an appropriate name in snake case for these attributes, and to provide additional metadata for these attributes.

At least one example of the element node will be provided along with some surrounding HTML providing necessary context. The target node to be analyzed will be delimited with an HTML comment.

An example of how to perform this task would be to give the name 'profile_url' (name in JSON response) to an href (attribute in JSON response) when it appears to be a link to the profile of a user account.

Additionally, for any href attributes, provide the following metadata:
1. is_page_link: Does the href value likely point to a new page or does it perform some sort of action or mutation? 
"##);
    let user_prompt = format!(r##"
Interpret these attributes:

---

{}

---

Example(s) of the element node which contain these attributes:

---

{}

---
"##, attributes, examples);
    log::debug!("user_prompt: {}", user_prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model": "gpt-4o-2024-08-06",
        "temperature": 0,
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": user_prompt
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "element_interpretation_response",
                "strict": true,
                "schema": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "attribute": {
                                "type": "string"
                            },
                            "name": {
                                "type": "string"
                            },
                            "is_page_link": {
                                "type": "string"
                            }
                        },
                        "required": ["attribute", "name"]
                    }
                }
            }
        }
    });
    let url = "https://api.openai.com/v1/chat/completions";
    let authorization = format!("Bearer {}", openai_api_key);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&request_json)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, authorization)
        .send()
        .await;
    let response = response.expect("Failed to send request to OpenAI");

    let json_response = response.json::<serde_json::Value>().await.expect("Unable to get JSON from response");
    log::debug!("json_response: {:?}", json_response);
    let json_response = json_response["choices"].as_array().unwrap();
    let json_response = &json_response[0]["message"]["content"].as_str().unwrap();

    let llm_element_data_response = serde_json::from_str::<Vec<LLMElementDataResponse>>(json_response)
        .expect("Could not parse JSON response as LLMElementDataResponse");
    log::debug!("llm_element_data_response: {:?}", llm_element_data_response);

    llm_element_data_response
        .iter()
        .map(|response| {
            NodeData {
                name: response.name.clone(),
                element: Some(ElementData {
                    attribute: response.attribute.clone(),
                    is_page_link: response.is_page_link.clone().unwrap_or(false),
                }),
                text: None,
            }
        })
        .collect()
}

pub async fn interpret_text_data(snippets: Vec<String>) -> NodeData {
    log::trace!("In interpret_text_data");

    assert!(snippets.len() > 0, "Did not receive any snippets");

    let examples = snippets.iter().enumerate().fold(
        String::new(),
        |mut acc, (index, snippet)| {
            acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
            acc
        }
    );

    unimplemented!()
}
