use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use std::env;
use sha2::{Sha256, Digest};
use bincode::{serialize, deserialize};
use std::sync::{Arc, OnceLock};
use std::collections::{HashSet};

use crate::node_data_structure::{RecursiveStructure};
use crate::node_data::{NodeData, ElementData, TextData};

static DB: OnceLock<Arc<sled::Db>> = OnceLock::new();

fn init_cache() -> Arc<sled::Db> {
    let db = sled::open("debug/cache").expect("Could not open cache");
    Arc::new(db)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMDataStructureResponse {
    #[serde(deserialize_with = "empty_string_as_none")]
    recursive_attribute: Option<String>,
    root_node_attribute_values: Option<Vec<String>>,
    #[serde(deserialize_with = "empty_string_as_none")]
    parent_node_attribute_value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMElementDataResponse {
    name: String,
    #[serde(default)]
    is_page_link: bool,
    is_peripheral_content: bool,
    is_advertisement: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMTextDataResponse {
    name: String,
    is_presentational: bool,
    is_primary_content: bool,
    is_peripheral_content: bool,
    is_advertisement: bool,
    is_title: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMAssociationsResponse {
    data: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMSummaryResponse {
    core_purpose: String,
}

pub async fn summarize_core_purpose(xml: String) -> String {
    log::trace!("In summarize_core_purpose");

    let system_prompt = format!(r##"
Your task is to summarize the core purpose of an HTML snippet.
"##);
    let user_prompt = format!(r##"
Snippet:
---

{}

---
"##, xml);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let llm_summary_response = serde_json::from_str::<LLMSummaryResponse>(&cached_response)
            .expect("Could not parse JSON response as LLMSummaryResponse");
        log::debug!("llm_summary_response: {:?}", llm_summary_response);

        return llm_summary_response.core_purpose;
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
                "name": "similarity_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "core_purpose": {
                            "type": "string"
                        }
                    },
                    "required": ["core_purpose"],
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

    let llm_summary_response = serde_json::from_str::<LLMSummaryResponse>(json_response)
        .expect("Could not parse JSON response as LLMSummaryResponse");
    log::debug!("llm_summary_response: {:?}", llm_summary_response);

    llm_summary_response.core_purpose
}

pub async fn interpret_associations(snippets: Vec<(String, String)>) -> Vec<Vec<String>> {
    log::trace!("In interpret_associations");

    assert!(snippets.len() > 0, "Did not receive any snippets");

    let examples = snippets.iter().enumerate().fold(
        String::new(),
        |mut acc, (_index, snippet)| {
            acc.push_str(&format!(r##"
Snippet Type ID: {}
Snippet content:
{}

"##, snippet.0, snippet.1));
            acc
        }
    );

    let system_prompt = format!(r##"
Your task is to match snippet type IDs to other snippet type IDs based on whether their content is related. Look for specific references or recurring themes, such as names, URLs, or keywords, that might signify a relationship between different snippets.

Potentially, several examples of a snippet of a particular type will be provided. One example of a snippet of one type might be related to just one example of another snippet of another type. In this case, provide a single group of both type IDs.

Provide your response as an array of arrays, where each array contains the snippet type IDs that are semantically or contextually related.
"##);
    let user_prompt = format!(r##"
---

{}

---
"##, examples);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let llm_associations_response = serde_json::from_str::<LLMAssociationsResponse>(&cached_response)
            .expect("Could not parse json response as LLMAssociationsResponse");
        log::debug!("llm_associations_response: {:?}", llm_associations_response);

        return postprocess_associations(llm_associations_response);
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
                "name": "similarity_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "array",
                            "items": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                }
                            }
                        }
                    },
                    "required": ["data"],
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

    let llm_associations_response = serde_json::from_str::<LLMAssociationsResponse>(json_response)
        .expect("Could not parse json response as LLMAssociationsResponse");
    log::debug!("llm_associations_response: {:?}", llm_associations_response);

    // Prompting here is fragile, so we do some manual cleanup
    postprocess_associations(llm_associations_response)
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> RecursiveStructure {
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
   • recursive_attribute: Provide XPath expression that selects the attribute that provides information about its recursive relationship to other such elements.
   • root_node_attribute_values: Provide possible values for recursive attributes that would signify that elements like this are root nodes.
   • parent_node_attribute_value: Provide an awk expression that would compute what the value of a recursive attribute would be for the parent node of a particular element if it is not a root node.
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

        return RecursiveStructure {
            recursive_attribute: llm_data_structure_response.recursive_attribute,
            root_node_attribute_values: llm_data_structure_response.root_node_attribute_values,
            parent_node_attribute_value: llm_data_structure_response.parent_node_attribute_value,
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
                        "recursive_attribute": {
                            "type": "string"
                        },
                        "root_node_attribute_values": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "parent_node_attribute_value": {
                            "type": "string"
                        }
                    },
                    "required": ["recursive_attribute", "root_node_attribute_values", "parent_node_attribute_value"],
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

    RecursiveStructure {
        recursive_attribute: llm_data_structure_response.recursive_attribute,
        root_node_attribute_values: llm_data_structure_response.root_node_attribute_values,
        parent_node_attribute_value: llm_data_structure_response.parent_node_attribute_value,
   }
}

async fn interpret_title(examples: String) -> NodeData {
    log::trace!("In interpret_title");

    let system_prompt = format!(r##"
Your task is to interpret the meaning of HTML title attributes and provide an appropriate generic name in snake case for these attributes. The attribute may have a variety of possible values so you should attempt to generalize as much as possible across all examples provided.

The title attribute to be interpreted will be found in an HTML element delimited with an HTML comment like so:
<!--Target node start --><div title="Example"><!--Target node end --></div>

At least one example of the target node will be provided, along with some surrounding HTML providing necessary context for you to interpret the meaning and purpose of the HTML target title. When multiple examples of a target node each containing a title are provided, treat the title contained within each target node as being the "same" and generalize as much as possible across its values with respect to the surrounding HTML.

An example of how to perform this task would be to give the name 'timestamp' to titles when all examples of it appear to be a timestamp.

Provide the following information in your response:
• name: A generic name in snake case that could be used to represent title values programmatically.
3. is_peripheral_content: Peripheral content is typically found in headers, footers, sidebars, banners, etc. and does not pertain to the core purpose of the website. Peripheral content is not the primary focus of the website's message or purpose. 
4. is_advertisement: Indicates if the value is an advertisement.

---
"##);
    let user_prompt = format!(r##"
Example(s) of the element node:

---

{}

---

"##, examples);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let response = serde_json::from_str::<LLMElementDataResponse>(&cached_response)
            .expect("Could not parse JSON response as LLMElementDataResponse");
        log::debug!("llm_element_data_response: {:?}", response);

        return NodeData {
            name: response.name.clone(),
            element: Some(ElementData {
                attribute: "title".to_string(),
                is_page_link: false,
                is_peripheral_content: response.is_peripheral_content.clone(),
                is_advertisement: response.is_advertisement.clone(),
            }),
            text: None,
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
                "name": "title_interpretation_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "is_peripheral_content": {
                            "type": "boolean"
                        },
                        "is_advertisement": {
                            "type": "boolean"
                        }
                    },
                    "required": ["name", "is_peripheral_content", "is_advertisement"],
                    "additionalProperties": false
                },
            },
        },
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

    let response = serde_json::from_str::<LLMElementDataResponse>(json_response)
        .expect("Could not parse JSON response as LLMElementDataResponse");
    log::debug!("llm_element_data_response: {:?}", response);

    NodeData {
        name: response.name.clone(),
        element: Some(ElementData {
            attribute: "title".to_string(),
            is_page_link: false,
            is_peripheral_content: response.is_peripheral_content.clone(),
            is_advertisement: response.is_advertisement.clone(),
        }),
        text: None,
    }
}

async fn interpret_href(examples: String) -> NodeData {
    log::trace!("In interpret_href");

    let system_prompt = format!(r##"
Your task is to interpret the meaning of HTML href attributes, provide an appropriate generic name in snake case for these attributes, and to provide additional metadata for these attributes. The attribute may have a variety of possible values so you should attempt to generalize as much as possible across all examples provided.

The href attribute to be interpreted will be found in an HTML element delimited with an HTML comment like so:
<!--Target node start --><a href="https://www.example.com"><!--Target node end --></a>

At least one example of the target node will be provided, along with some surrounding HTML providing necessary context for you to interpret the meaning and purpose of the HTML target href. When multiple examples of a target node each containing an href are provided, treat the href contained within each target node as being the "same" and generalize as much as possible across its values with respect to the surrounding HTML.

An example of how to perform this task would be to give the name 'profile_url' to an href when all examples of it appear to be a link to the profile page of a user account.

Provide the following information in your response:
1. name: A generic name in snake case that could be used to represent href values programmatically.
2. is_page_link: Indicate whether these href(s) likely just point to a new page or if they are for performing some sort of action or mutation. An example of a page link is a link to a another page in a website where more content is consumed such as visiting an 'about' page from the landing page. Action/mutation href likely require a login and change something about the state of an item on the website.
3. is_peripheral_content: Peripheral content is typically found in headers, footers, sidebars, banners, etc. and does not pertain to the core purpose of the website. Peripheral content is not the primary focus of the website's message or purpose. 
4. is_advertisement: Indicates if the value is an advertisement.

---
"##);
    let user_prompt = format!(r##"
Example(s) of the element node:

---

{}

---

"##, examples);
    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);

    let hash = compute_hash(vec![system_prompt.clone(), user_prompt.clone()]);
    if let Some(cached_response) = get_cached_response(hash.clone()) {
        log::info!("Cache hit!");

        let response = serde_json::from_str::<LLMElementDataResponse>(&cached_response)
            .expect("Could not parse JSON response as LLMElementDataResponse");
        log::debug!("llm_element_data_response: {:?}", response);

        return NodeData {
            name: response.name.clone(),
            element: Some(ElementData {
                attribute: "href".to_string(),
                is_page_link: response.is_page_link.clone(),
                is_peripheral_content: response.is_peripheral_content.clone(),
                is_advertisement: response.is_advertisement.clone(),
            }),
            text: None,
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
                "name": "href_interpretation_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "is_page_link": {
                            "type": "boolean"
                        },
                        "is_peripheral_content": {
                            "type": "boolean"
                        },
                        "is_advertisement": {
                            "type": "boolean"
                        }
                    },
                    "required": ["name", "is_page_link", "is_peripheral_content", "is_advertisement"],
                    "additionalProperties": false
                },
            },
        },
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

    let response = serde_json::from_str::<LLMElementDataResponse>(json_response)
        .expect("Could not parse JSON response as LLMElementDataResponse");
    log::debug!("llm_element_data_response: {:?}", response);

    NodeData {
        name: response.name.clone(),
        element: Some(ElementData {
            attribute: "href".to_string(),
            is_page_link: response.is_page_link.clone(),
            is_peripheral_content: response.is_peripheral_content.clone(),
            is_advertisement: response.is_advertisement.clone(),
        }),
        text: None,
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

    let futures = meaningful_attributes.iter().map(|attribute| {
        let examples_clone = examples.clone();
        async move {
            match attribute.as_str() {
                "href" => interpret_href(examples_clone).await,
                "title" => interpret_title(examples_clone).await,
                _ => panic!("Unexpected attribute: {}", attribute),
            }
        }
    });

    futures::future::join_all(futures).await
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
Your task is to interpret the meaning of HTML text nodes, provide an appropriate name in snake case that could be used for programmatically representing this type of data, and to provide additional metadata for these text nodes.

At least one example of the text node will be provided along with some surrounding HTML providing necessary context. The target node to be analyzed will be delimited with an HTML comment.

An example of how to perform this task would be to give the name 'comment_text' (name in your JSON response) to a text node when it appears to represent a user-generated comment on a website.

Additionally, provide this metadata in your JSON response:
1. is_presentational: Indicates if the text primarily serves a visual or structural role without adding meaningful data context. For example, if a text node is used to delineate other HTML nodes, it is presentational, but if a text node contains meaningful natural language meant for people to read, it is not presentational.
2. is_primary_content: Primary content is the main information or core purpose of a web page, often the reason users visit the site and includes closely-related metadata. Headings, article text would be examples of primary content. Various links to unrelated  or vaguely-related pages would be examples of non-primary content.
3. is_peripheral_content: Peripheral content is typically found in headers, footers, sidebars, banners, etc. and does not pertain to the core purpose of the website. Peripheral content is not the primary focus of the website's message or purpose. 
4. is_advertisement: Indicates if the text is an advertisement.
5. is_title: Despite how it currently gets rendered based on the HTML, does this text node fulfill the typical purpose of titles or headings? Is this text node a prominent focal point that draws the user's attention?
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
                is_peripheral_content: llm_text_data_response.is_peripheral_content.clone(),
                is_advertisement: llm_text_data_response.is_advertisement.clone(),
                is_title: llm_text_data_response.is_title.clone(),
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
                        },
                        "is_peripheral_content": {
                            "type": "boolean"
                        },
                        "is_advertisement": {
                            "type": "boolean"
                        },
                        "is_title": {
                            "type": "boolean"
                        }
                    },
                    "required": ["name", "is_presentational", "is_primary_content", "is_peripheral_content", "is_advertisement", "is_title"],
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
            is_peripheral_content: llm_text_data_response.is_peripheral_content.clone(),
            is_advertisement: llm_text_data_response.is_advertisement.clone(),
            is_title: llm_text_data_response.is_title.clone(),
        }),
    }
}

fn get_cached_response(key: String) -> Option<String> {
    let db = DB.get_or_init(init_cache);
    match db.get(key).expect("Could not get value from cache") {
        Some(data) => Some(deserialize(&data).expect("Could not deserialize data")),
        None => None,
    }
}

fn set_cached_response(key: String, value: String) {
    let db = DB.get_or_init(init_cache);
    db.insert(key, serialize(&value).expect("Could not serialize data")).expect("Could not store value in cache");
}

fn compute_hash(hasher_items: Vec<String>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(hasher_items.join(""));
    format!("{:x}", hasher.finalize())
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

fn postprocess_associations(response: LLMAssociationsResponse) -> Vec<Vec<String>> {
    response   
        .data
        .into_iter()
        .filter_map(|inner_vec| {
            let unique_items: HashSet<String> = inner_vec.into_iter().collect();
            let unique_vec: Vec<String> = unique_items.into_iter().collect();

            if unique_vec.len() > 1 {
                Some(unique_vec)
            } else {
                None
            }
        })
        .collect()
}

