use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::rc::{Rc};

use crate::node_data::{NodeData};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialNodeData {
    pub xpath: String,
    pub name: String,
}

pub async fn interpret_node(fields: String, context: String) -> Result<String, ()> {
    log::trace!("In interpret_node");

    assert!(!fields.is_empty());
    assert!(!context.is_empty());

    let prompt = format!(r##"
I would like you to come up with an appropriate type name for the following set of fields:


{}

---

The context is which these field(s) appear which you should consider when naming fields:

{}

---

Please provide your response as a single Pascal case string with no commentary, introduction or summary. Thank you.
"##, fields, context);
    log::trace!("prompt: {}", prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model":  "gpt-4-0125-preview",
        "temperature":  0,
        "messages":  [
            {
                "role": "user",
                "content": prompt
            }
        ]
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
    log::trace!("response: {:?}", response);
    let response = response.expect("Failed to send request to OpenAI");

    let json_response = response.json::<serde_json::Value>().await.expect("Unable to get JSON from response");
    log::debug!("json_response: {:?}", json_response);
    let json_response = json_response["choices"].as_array().unwrap();
    let json_response = &json_response[0]["message"]["content"].as_str().unwrap();
    let json_response = fix_json_response(json_response);
    log::debug!("json_response: {:?}", json_response);

    Ok(json_response.to_string())
}

pub async fn generate_node_data(xml: String) -> Result<Vec<NodeData>, ()> {
    log::trace!("In generate_node_data");

    let prompt = format!(r##"
I'm analyzing an HTML/XML snippet to extract important non-presentational data elements that a user would care about.
For each significant piece of information in the snippet, I want you to provide the following:

1. The XPath expression that can be used to select this information.
2. A suitable name in snake case that can be used to represent the data programmatically.

Here is the HTML/XML text I'm examining:

---

{}

---

Anticipate the possibility that there might not be any significant information in the XML, in which case return an empty JSON array.
If the snippet seems to contain an ID or similar dynamically-generated value, ensure that corresponding xpath expression is generic with respect to the value.
Otherwise, please provide your response as an array of JSON objects that look like this:

{{
    "xpath": "/div/tr/*",
    "name": "url"
}}

And do not include any commentary, introduction or summary. Thank you.
"##, xml);
    log::trace!("prompt: {}", prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model":  "gpt-3.5-turbo-0125",
        "temperature":  0,
        "messages":  [
            {
                "role": "user",
                "content": prompt
            }
        ]
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
    log::trace!("response: {:?}", response);
    let response = response.expect("Failed to send request to OpenAI");

    let json_response = response.json::<serde_json::Value>().await.expect("Unable to get JSON from response");
    log::debug!("json_response: {:?}", json_response);
    let json_response = json_response["choices"].as_array().unwrap();
    let json_response = &json_response[0]["message"]["content"].as_str().unwrap();
    let json_response = fix_json_response(json_response);
    log::debug!("json_response: {:?}", json_response);

    let partial_node_data = serde_json::from_str::<Vec<PartialNodeData>>(json_response)
        .expect("Could not marshal response to PartialNodeData");

    let node_data: Vec<NodeData> = partial_node_data.iter().map(|item| {
        NodeData {
            xpath: Some(item.xpath.to_string()),
            name: item.name.to_string(),
            value: None,
        }
    }).collect();

    Ok(node_data)
}

fn fix_json_response(response: &str) -> &str {
    let without_backticks = remove_codeblock_delimiters(response);
    let without_label = remove_label_text(without_backticks);

    return without_label;
}

fn remove_codeblock_delimiters(s: &str) -> &str {
     if s.starts_with("```") && s.ends_with("```") {
         s.trim_start_matches("```").trim_end_matches("```")
     } else {
         s
     }
}

fn remove_label_text(s: &str) -> &str {
    if s.starts_with("json") {
        return &s[4..];
    }

    s
}
