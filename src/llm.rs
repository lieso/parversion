use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use crate::node_data::{NodeData, ElementNodeMetadata, TextNodeMetadata};
use crate::node_data_structure::{NodeDataStructure};
use crate::xml::{Xml};

fn default_is_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialElementNodeMetadata {
    pub attribute: String,
    pub new_name: String,
    #[serde(default = "default_is_false")]
    pub is_id: bool,
    #[serde(default = "default_is_false")]
    pub is_url: bool,
    #[serde(default = "default_is_false")]
    pub is_page_link: bool,
    #[serde(default = "default_is_false")]
    pub is_action_link: bool,
    #[serde(default = "default_is_false")]
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PartialTextNodeMetadata {
    pub name: String,
    pub is_semantically_significant: bool,
    pub is_page_action: bool,
    pub is_presentational: bool,
    pub is_primary_content: bool,
    pub is_main_primary_content: bool,
}

pub async fn xml_to_data(xml: &Xml, surrounding_xml: String, examples: Vec<String>) -> Result<Vec<NodeData>, ()> {
    log::trace!("In xml_to_data");

    if xml.is_element() {
        return element_to_data(xml, surrounding_xml, examples).await;
    } else {
        return text_to_data(xml, surrounding_xml, examples).await;
    }
}

pub async fn xml_to_data_structure(xml: &Xml, surrounding_xml: String, examples: Vec<String>) -> Result<Vec<NodeDataStructure>, ()> {
    log::trace!("In xml_to_data_structure");

    assert!(!xml.is_text(), "Did not expect to receive a text node");

    let examples_message: String = if examples.is_empty() {
        "".to_string()
    } else {
        examples.iter().enumerate().fold(
            format!(r##"
The following are examples of this element node as it appears in other sections of the web page or in other versions of the web page. Use this to help you complete your task."##),
            |mut acc, (index, example)| {
                acc.push_str(&format!(r##"

Example {}:
{}"##, index + 1, example));
                acc
            })
    };

    let prompt = format!(r##"
Your job is to examine an HTML element node and to infer its relationship to other nodes such as when an element node (along with its children) is actually a member of a list of items that gets rendered to a user or if there is a recursive relationship to other nodes. For example, a website might render a list of weather forecasts for a particular city with one element corresponding to a forecast for one day and the next item being the forecast for the following day. A website may also render a discussion thread consisting of replies which would be an example of a recursive relationship where each item here has a parent relationship to another item (unless it is a root reply).

Do your best to determine if any of the following relationships apply to the element node I will provide you. It's possible for multiple relationships to apply to a single element and you should anticipate the possibility that none of these may apply to the node:

1. Does the element represent a recursive relationship to other elements? If so, please provide the following:
   • root_node_traversal_direction: to determine if such a node is a root node, would we need to traverse HTML element siblings (Sibling), traverse upward parent elements (Up) or children (Child)?
   • root_node_tag_name: provide HTML element node tag name of element that would tell you if the current node is a root node
   • root_node_attributes: provide all HTML element node attribute names of element that would tell if you if the current node is a root node
   • root_node_target_values: if applicable, provide all HTML element node attribute names and the specific corresponding values that would can be used to check if current node is a root node
   • parent_node_traversal_direction: to get to a parent node (for non-root nodes), would we need to traverse HTML element siblings (Sibling), traverse upward parent elements (Up) or children (Child)?
   • parent_node_tag_name: provide HTML element node tag name of a parent
   • parent_node_attributes: provide all HTML element node attribute names of a parent
   • parent_node_target_values: if applicable, provide all HTML element node attribute names and the specific corresponding values that would identify a parent node
2. Does the element represent an item in list? If so, please provide the following:
   • next_node_traversal_direction: to get to the next item in the, would be need to traverse HTML element siblings (Sibling), traverse upward parent elements (Up) or children (Child)?
   • next_node_tag_name: what is the HTML element node tag name of the next item in the list?
   • next_node_attributes: provide all HTML element node attribute names of the next item in the list

Here is the HTML element node for you to examine:

---

{}

---

This is the surrounding HTML in which the element node appears (the element node is indicated with an HTML comment):

---

{}

---

{}

Provide your response as JSON using the above snake case criteria as JSON keys.

Please do not include any commentary, introduction or summary. Thank you."##, xml.to_string(), surrounding_xml, examples_message);
    log::debug!("prompt: {}", prompt);

    Ok(Vec::new())
}

async fn element_to_data(xml: &Xml, surrounding_xml: String, examples: Vec<String>) -> Result<Vec<NodeData>, ()> {
    log::trace!("In element_to_data");
    
    let examples_message: String = if examples.is_empty() {
        "".to_string()
    } else {
        examples.iter().enumerate().fold(
            format!(r##"
The following are examples of this element node as it appears in other sections of the web page or in other versions of the web page. Use this to help you complete your task."##),
            |mut acc, (index, example)| {
                acc.push_str(&format!(r##"

Example {}:
{}"##, index + 1, example));
                acc
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
2. A new suitable name in snake case that could be used to represent this data programmatically. For example, it makes sense for href attributes to take a name containing the text 'url' plus any additional context.
3. If the attribute value is an ID
4. If the attribute value is a URL (is_url). For each URL, if present, classify each link as either "page link" (is_page_link) if it leads to another page (e.g., content pages, informational pages), or "action link" if it performs an action that mutates something (e.g., form submissions, deletion actions) (is_action_link)
5. If the attribute value is part of the primary content. Primary content is the main information or core purpose of the web page, often the reason users visit the site, and includes closely-related metadata (is_primary_content)

Here is the HTML element node for you to examine:

---

{}

---

This is the surrounding HTML in which the element node appears (the element node is indicated with an HTML comment):

---

{}

---

{}

Please provide your response as an array of JSON objects that look like this:

{{
    "attribute": "href",
    "new_name": "icon_url",
    "is_id": false,
    "is_page_link": false,
    "is_action_link": false,
    "is_primary_content": false
}}

Anticipate the possibility that there might not be any significant information in the XML, in which case return an empty JSON array. Do no include any commentary, introduction or summary. Thank you."##, xml.to_string(), surrounding_xml, examples_message);
    log::debug!("prompt: {}", prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model":  "gpt-4o",
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

    let partial_node_data = serde_json::from_str::<Vec<PartialElementNodeMetadata>>(json_response)
        .expect("Could not marshal response to PartialElementNodeMetadata");

    let node_data: Vec<NodeData> = partial_node_data.iter().map(|item| {
        NodeData {
            name: item.new_name.clone(),
            element_fields: Some(ElementNodeMetadata {
                attribute: item.attribute.clone(),
                is_id: item.is_id.clone(),
                is_url: item.is_url.clone(),
                is_page_link: item.is_page_link.clone(),
                is_action_link: item.is_action_link.clone(),
                is_primary_content: item.is_primary_content.clone(),
            }),
            text_fields: None,
        }
    }).collect();

    Ok(node_data)
}

async fn text_to_data(xml: &Xml, surrounding_xml: String, examples: Vec<String>) -> Result<Vec<NodeData>, ()> {
    log::trace!("text_to_data");
    
    let examples_message: String = if examples.is_empty() {
        "".to_string()
    } else {
        examples.iter().enumerate().fold(
            format!(r##"
The following are examples of this text node as it appears in other sections of the web page or in other versions of the web page. Use this to help you complete your task."##),
            |mut acc, (index, example)| {
                acc.push_str(&format!(r##"

Example {}:
{}"##, index + 1, example));
                acc
            })
    };

    let prompt = format!(r##"
Your job is to reverse engineer the data model for a rendered HTML text node. I will provide the surrounding HTML which should help you to determine the appropriate classification.

Please provide the following:

1 An appropriate variable name in snake case that could be used to represent this data programmatically. For example, strings containing integers may take a name like 'order' if
  the surrounding HTML appears to be rendering a list of items in a particular order.
2 Determine the properties of the text node:
   • is_semantically_significant: Indicates if the text conveys meaningful, data-related information, adding to the understanding of the context in which it appears (true or false).
   • is_presentational: Indicates if the text primarily serves a visual or structural role without adding meaningful data context. This is for elements used to visually format or structure the page (true or false).
3 Specify if the text node represents an in-page action (e.g., links like "hide" or "submit"):
   • is_page_action: These are non-informational, action-oriented text nodes that do not represent the primary content of the document but instead assist the reader in using the
     website (true/false).
4 If the text node is part of the primary content. Primary content is the main information or core purpose of the web page, often the reason users visit the site, and includes closely-related metadata (is_primary_content)
5 If the text node is part of the main primary content. This does not include closely-related metadata which is typically less prominent or greyed out, while the main primary content is often larger with a more contrasting font (is_main_primary_content).


Here is the HTML text node for you to examine:

---

{}

---

This is the surrounding HTML in which the text node appears (the text node is indicated with an HTML comment):

---

{}

---

{}

Please provide your response as a JSON object that looks like this:

{{
    "name": "reply",
    "is_semantically_significant": true,
    "is_page_action": false,
    "is_presentational": true,
    "is_primary_content": false,
    "is_main_primary_content": false
}}

And do not include any commentary, introduction or summary. Thank you."##, xml, surrounding_xml, examples_message);
    log::debug!("prompt: {}", prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model":  "gpt-4o",
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

    let partial_node_data = serde_json::from_str::<PartialTextNodeMetadata>(json_response)
        .expect("Could not marshal response to PartialTextNodeMetadata");

    let node_data = NodeData {
        name: partial_node_data.name,
        text_fields: Some(TextNodeMetadata {
            is_informational: partial_node_data.is_semantically_significant && !partial_node_data.is_page_action && !partial_node_data.is_presentational,
            is_primary_content: partial_node_data.is_primary_content.clone(),
            is_main_primary_content: partial_node_data.is_main_primary_content.clone(),
        }),
        element_fields: None,
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
