use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use std::env;

use crate::node_data_structure::{NodeDataStructure};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LLMDataStructureResponse {
    #[serde(deserialize_with = "empty_string_as_none")]
    root_node_xpath: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    parent_node_xpath: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    next_item_xpath: Option<String>,
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

pub async fn interpret_data_structure(snippets: Vec<String>) -> Vec<NodeDataStructure> {
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
    log::debug!("json_response: {:?}", json_response);

    let llm_data_structure_response = serde_json::from_str::<LLMDataStructureResponse>(json_response)
        .expect("Could not parse json response as LLMDataStructureResponse");

    log::debug!("llm_data_structure_response: {:?}", llm_data_structure_response);

    //[2024-08-12T01:04:07Z DEBUG parversion::llm] json_response: "{\"next_item_xpath\":\"following-sibling::tr[@class='athing comtr'][1]\",\"parent_node_xpath\":\"preceding-sibling::tr[@class='athing comtr'][1]\",\"root_node_xpath\":\"@indent='0'\"}"
    //[2024-08-12T01:04:07Z DEBUG parversion::llm] llm_data_structure_response: LLMDataStructureResponse { root_node_xpath: Some("@indent='0'"), parent_node_xpath: Some("preceding-sibling::tr[@class='athing comtr'][1]"), next_item_xpath: Some("following-sibling::tr[@class='athing comtr'][1]") }

    unimplemented!()
}
