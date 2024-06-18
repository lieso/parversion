use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use crate::node_data::{NodeData, ElementNodeData, TextNodeData};
use crate::xml::{Xml};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialElementNodeData {
    pub attribute: String,
    pub new_name: String,
    pub is_id: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialTextNodeData {
    pub name: String,
    pub is_informational: bool,
}

pub async fn xml_to_data(xml: &Xml, surrounding_xml: &Xml, examples: Vec<&Xml>) -> Result<Vec<NodeData>, ()> {
    log::trace!("In xml_to_data");

    if xml.is_element() {
        return element_to_data(xml, surrounding_xml, examples);
    } else {
        return text_to_data(xml, surrounding_xml, examples);
    }
}

async fn element_to_data(xml: String, surrounding_xml: &Xml, examples: Vec<&Xml>) -> Result<Vec<NodeData>, ()> {
    log::trace!("In element_to_data");
    
    let examples_message = if examples.is_empty() {
        ""
    } else {
        examples.iter().enumerate().fold(
            format!(r##"
The following are examples of this element node as it appears in other sections of the web page or in other versions of the web page. Use this to help you complete your task."##),
            |mut acc, (index, example)| {
                acc.push_str(format!(r##"

Example {}:
{}"##, index + 1, example))
            })
    };

    let prompt = format!(r##"
Your job is to reverse engineer the data model for a rendered HTML element. This task involves extracting salient information from HTML attributes, while ignoring presentational aspects like styling, decoration, or accessibility enhancements that do not affect the underlying data model.

Disregard anything that does not contribute to the data model, including the following:
1. Presentation-related content (such as inline CSS or style-related classes)
2. Browser specific meta tags which provide metadata to the browser rather than intrinsic information about the web page's underlying content and structure.
3. Any loading of additional code (JavaScript, CSS) which are commonly found in script or link elements

For each informative attribute, I want you to provide the following:

1. The attribute name
2. A new suitable name in snake case that could be used to represent this data programmatically. For example, it makes sense for href attributes to take a name containing the text 'url' plus and additional context.
3. If the attribute value is an ID

Here is the HTML element node for you to examine:

---

{}

---

This is the surrounding HTML in which the element node appears:

---

{}

---

{}

Please provide your response as an array of JSON objects that looks like this:

{{
    "attribute": "href",
    "new_name": "icon_url",
    "is_id": false
}}

Anticipate the possibility that there might not be any significant information in the XML, in which case return an empty JSON array. Do no include any commentary, introduction or summary. Thank you."##, xml, surrounding_xml, examples);
    log::debug!("prompt: {}", prompt);

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

    let partial_node_data = serde_json::from_str::<Vec<PartialElementNodeData>>(json_response)
        .expect("Could not marshal response to PartialElementNodeData");

    let node_data: Vec<NodeData> = partial_node_data.iter().map(|item| {
        NodeData {
            name: item.new_name,
            element_fields: Some(ElementNodeData {
                attribute: item.attribute,
                is_id: item.is_id,
            }),
        }
    }).collect();

    Ok(node_data)
}

async fn text_to_data(xml: String, surrounding_xml: String, examples: Vec<String>) -> Result<Vec<NodeData>, ()> {
    log::trace!("text_to_data");
    
    let examples_message = if examples.is_empty() {
        ""
    } else {
        examples.iter().enumerate().fold(
            format!(r##"
The following are examples of this text node as it appears in other sections of the web page or in other versions of the web page. Use this to help you complete your task."##),
            |mut acc, (index, example)| {
                acc.push_str(format!(r##"

Example {}:
{}"##, index + 1, example))
            })
    };

    let prompt = format!(r##"
Your job is to reverse engineer the data model for a rendered HTML text node. I will provide the surrounding HTML which should help you to determine the context in which this text appears.

Please provide the following:

1. An appropriate variable name in snake case that could be used to represent this data programmatically. For example strings containing integers may take a name like 'order' if the surrounding HTML appears to be rendering a list of items in a particular order.
2. If the value is informational. For example, a text node that is just a standalone pipe symbol may be used to visually separate content, but doesn't contribute to the underlying data model of the web page so it is not informational.

Here is the HTML text node for you to examine:

---

{}

---

This is the surrounding HTML in which the text node appears:

---

{}

---

{}

Please provide your response as a JSON object that looks like this:

{{
    "name": "reply",
    "is_informational": true
}}

And do not include any commentary, introduction or summary. Thank you."##, xml, surrounding_xml, examples_message);
    log::debug!("prompt: {}", prompt);

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

    let partial_node_data = serde_json::from_str::<PartialTextNodeData>(json_response)
        .expect("Could not marshal response to PartialTextNodeData");

    let node_data = NodeData {
        name: partial_node_data.name,
        text_fields: Some(TextNodeData {
            is_informational: partial_node_data.is_informational,
        }),
    };

    Ok(vec![node_data])
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
