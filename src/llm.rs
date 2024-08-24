use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use std::env;
use sha2::{Sha256, Digest};
use bincode::{serialize, deserialize};

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
struct LLMElementDataResponseItem {
    attribute: String,
    name: String,
    is_page_link: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMElementDataResponse {
    attributes: Vec<LLMElementDataResponseItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMTextDataResponse {
    name: String,
    is_presentational: bool,
    is_primary_content: bool,
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
   • root_node_xpath: Provide a complete and generic XPath expression relative to the element node that would test if elements of this type are root nodes. 
   • parent_node_xpath: Provide complete and generic XPath expression that would select the element node's parent if it is not a root node.
2. Does the element represent an item in a meaningful list? If so, please provide the following:
   • next_item_xpath: Provide complete and generic XPath expression that would select the next item in the list.

Ensure that each XPath expression specifies the traversal direction. Do not overfit to the examples provided, you must ensure the XPath expression is generic, reusable and can be applied to similar nodes.

"##);
    let user_prompt = format!(r##"
Example(s) of the node to be analyzed:

---

{}

---
"##, examples);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let llm_data_structure_response = serde_json::from_str::<LLMDataStructureResponse>(&cached_response)
            .expect("Could not parse json response as LLMDataStructureResponse");
        log::debug!("llm_data_structure_response: {:?}", llm_data_structure_response);

        return NodeDataStructure {
            root_node_xpath: llm_data_structure_response.root_node_xpath,
            parent_node_xpath: llm_data_structure_response.parent_node_xpath,
            next_item_xpath: llm_data_structure_response.next_item_xpath,
       };
    } 

    log::info!("Cache miss!");

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

    set_cached_response(hash.clone(), json_response.to_string());

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
Your task is to interpret the meaning of HTML element attributes, provide an appropriate generic name in snake case for these types of attributes, and to provide additional metadata for these attributes. The attribute may have a variety of possible values so you should attempt to generalize as much as possible across all examples provided.

At least one example of the element node will be provided along with some surrounding HTML providing necessary context. The target node to be analyzed will be delimited with an HTML comment.

An example of how to perform this task would be to give the name 'profile_url' (name in JSON response) to an href (attribute in JSON response) when it appears to be a link to the profile of a user account. Each item in your JSON array response must correspond to one HTML attribute; so if two attributes are provided, there should only be two items in the JSON array response. When providing the interpretation for a particular attribute (key 'attribute' in JSON response), only supply the attribute name. An example of a JSON response when two attributes are provided for interpretation:
{{
    attributes: [
        {{
            attribute: "href",
            name: "profile_url",
            is_page_link: true
        }},
        {{
            attribute: "title",
            name: "timestamp",
            is_page_link: false
        }}
    ]
}}

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
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let llm_element_data_response = serde_json::from_str::<LLMElementDataResponse>(&cached_response)
            .expect("Could not parse JSON response as LLMElementDataResponse");
        log::debug!("llm_element_data_response: {:?}", llm_element_data_response);

        return llm_element_data_response
            .attributes
            .iter()
            .map(|response| {
                NodeData {
                    name: response.name.clone(),
                    element: Some(ElementData {
                        attribute: response.attribute.clone(),
                        is_page_link: response.is_page_link.clone(),
                    }),
                    text: None,
                }
            })
            .collect();
    }

    log::info!("Cache miss!");

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
                    "type": "object",
                    "properties": {
                        "attributes": {
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
                                        "type": "boolean"
                                    }
                                },
                                "required": ["attribute", "name", "is_page_link"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["attributes"],
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

    set_cached_response(hash.clone(), json_response.to_string());

    let llm_element_data_response = serde_json::from_str::<LLMElementDataResponse>(json_response)
        .expect("Could not parse JSON response as LLMElementDataResponse");
    log::debug!("llm_element_data_response: {:?}", llm_element_data_response);

    llm_element_data_response
        .attributes
        .iter()
        .map(|response| {
            NodeData {
                name: response.name.clone(),
                element: Some(ElementData {
                    attribute: response.attribute.clone(),
                    is_page_link: response.is_page_link.clone(),
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

    let system_prompt = format!(r##"
Your task is to interpret the meaning of HTML text nodes, provide an appropriate name in snake case that could be used for programmtically representing this type of data, and to provide additional metadata for these text nodes.

At least one example of the text node will be provided along with some surrounding HTML providing necessary context. The target node to be analyzed will be delimited with an HTML comment.

An example of how to perform this task would be to give the name 'comment_text' (name in your JSON response) to a text node when it appears to represent a user-generated comment on a website.

Additionally, provide this metadata in your JSON response:
1. is_presentational: Indicates if the text primarily serves a visual or structural role without adding meaningful data context. For example, if a text node is used to delineate other HTML nodes, it is presentational, but if a text node contains meaningful natural language meant for people to read, it is not presentational.
2. is_primary_content: Primary content is the main information or core purpose of a web page, often the reason users visit the site and includes closely-related metadata. Headings, article text would be examples of primary content. Various links to unrelated  or vaguely-related pages would be examples of non-primary content.
"##);
    let user_prompt = format!(r##"
Examples(s) of the text node to be analyzed:

---

{}

---
"##, examples);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let llm_text_data_response = serde_json::from_str::<LLMTextDataResponse>(&cached_response)
            .expect("Could not parse JSON response as LLMTextDataResponse");
        log::debug!("llm_text_data_response: {:?}", llm_text_data_response);

        return NodeData {
            name: llm_text_data_response.name.clone(),
            element: None,
            text: Some(TextData {
                is_presentational: llm_text_data_response.is_presentational.clone(),
                is_primary_content: llm_text_data_response.is_primary_content.clone(),
            }),
        };
    }

    log::info!("Cache miss!");

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
                "name": "text_interpretation_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "is_presentational": {
                            "type": "boolean"
                        },
                        "is_primary_content": {
                            "type": "boolean"
                        }
                    },
                    "required": ["name", "is_presentational", "is_primary_content"],
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

    set_cached_response(hash.clone(), json_response.to_string());

    let llm_text_data_response = serde_json::from_str::<LLMTextDataResponse>(json_response)
        .expect("Could not parse JSON response as LLMTextDataResponse");
    log::debug!("llm_text_data_response: {:?}", llm_text_data_response);

    NodeData {
        name: llm_text_data_response.name.clone(),
        element: None,
        text: Some(TextData {
            is_presentational: llm_text_data_response.is_presentational.clone(),
            is_primary_content: llm_text_data_response.is_primary_content.clone(),
        }),
    }
}

fn get_cached_response(key: String) -> Option<String> {
    let db = sled::open("debug/cache").expect("Could not connect to cache");
    match db.get(key).expect("Could not get value from cache") {
        Some(data) => Some(deserialize(&data).expect("Could not deserialize data")),
        None => None,
    }
}

fn set_cached_response(key: String, value: String) {
    let db = sled::open("debug/cache").expect("Could not connect to cache");
    db.insert(key, serialize(&value).expect("Could not serialize data")).expect("Could not store value in cache");
}

fn compute_hash(hasher_items: Vec<String>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(hasher_items.join(""));
    format!("{:x}", hasher.finalize())
}
