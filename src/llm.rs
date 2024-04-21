use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use crate::models::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialNodeData {
    pub xpath: String,
    pub is_url: bool,
    pub name: String,
}

//pub async self_improve/reconcile twp different result same xpath

pub async fn generate_node_data(xml: String) -> Result<Vec<NodeData>, ()> {
    log::trace!("In generate_node_data");

    let prompt = format!(r##"
I'm analyzing an HTML/XML snippet to extract important data elements that a user would see or use. For each significant element, I want you to provide the following:

1. The XPath expression that can be used to select the element.
2. A suitable name in snake case that can be used to represent the data programmatically.
3. Whether the data is a URL, absolute or relative

Here is the HTML/XML text I'm examining:

---

{}

---

Anticipate the possibility that there might not be any significant information in the XML, in which case return an empty JSON array. Otherwise, please provide your response as an array of JSON objects that look like this:

{{
    "xpath": "/div/tr/*",
    "is_url": true,
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
            xpath: item.xpath.to_string(),
            is_url: item.is_url,
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
