use reqwest::header;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::env;
use std::sync::Arc;
use sled::Db;
use once_cell::sync::Lazy;
use sha2::{Sha256, Digest};


use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::config::{CONFIG};

static DB: Lazy<Arc<Db>> = Lazy::new(|| {
    let debug_dir = "path_to_debug_dir"; // Replace with actual path or configuration
    let db = sled::open(format!("{}/cache", debug_dir)).expect("Could not open cache");
    Arc::new(db)
});

pub struct OpenAI;

impl OpenAI {
    pub async fn get_field_transformation(
        field: String,
        snippet: String,
    ) -> FieldTransformation {
        log::trace!("In get_field_transformation");

        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");

        log::debug!("field: {:?}", field);
        log::debug!("snippet: {}", snippet);

        // Implement the logic for this function
        unimplemented!()
    }

    async fn send_openai_request<T>(
        system_prompt: String,
        user_prompt: String,
        response_format: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let hash = Self::compute_hash(vec![system_prompt.clone(), user_prompt.clone(), response_format.to_string()]);

        let response = Self::get_or_set_cache(hash.clone(), || async {
            let openai_api_key = env::var("OPENAI_API_KEY").ok()?;
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
                "response_format": response_format,
            });

            let url = "https://api.openai.com/v1/chat/completions";
            let authorization = format!("Bearer {}", openai_api_key);
            let client = reqwest::Client::new();

            match client
                .post(url)
                .json(&request_json)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, authorization)
                .send()
                .await
            {
                Ok(res) => {
                    let json_response = res.json::<serde_json::Value>().await.ok()?;
                    json_response["choices"].as_array().and_then(|choices| {
                        choices.get(0).and_then(|choice| choice["message"]["content"].as_str().map(String::from))
                    })
                }
                Err(_) => None,
            }
        }).await;

        let json_response = response.ok_or("Failed to get response from OpenAI")?;
        let parsed_response: T = serde_json::from_str(&json_response)?;
        Ok(parsed_response)
    }

    fn compute_hash(hasher_items: Vec<String>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(hasher_items.join(""));
        format!("{:x}", hasher.finalize())
    }

    async fn get_or_set_cache<F, Fut>(hash: String, fetch_data: F) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        if let Some(cached_response) = Self::get_cached_response(hash.clone()) {
            log::info!("Cache hit!");
            Some(cached_response)
        } else {
            log::info!("Cache miss!");
            if let Some(response) = fetch_data().await {
                Self::set_cached_response(hash, response.clone());
                Some(response)
            } else {
                None
            }
        }
    }

    fn get_cached_response(key: String) -> Option<String> {
        let db = DB.clone();
        match db.get(key).expect("Could not get value from cache") {
            Some(data) => Some(String::from_utf8(data.to_vec()).expect("Could not deserialize data")),
            None => None,
        }
    }

    fn set_cached_response(key: String, value: String) {
        let db = DB.clone();
        db.insert(key, value.into_bytes()).expect("Could not store value in cache");
    }
}









//async fn interpret_href(examples: String, core_purpose: String) -> NodeData {
//    log::trace!("In interpret_href");
//
//    let system_prompt = format!(r##"
//Your task is to interpret the meaning of HTML href attributes, provide an appropriate generic name in snake case for these attributes, and to provide additional metadata for these attributes. The attribute may have a variety of possible values so you should attempt to generalize as much as possible across all examples provided.
//
//The href attribute to be interpreted will be found in an HTML element delimited with an HTML comment like so:
//<!--Target node start --><a href="https://www.example.com"><!--Target node end --></a>
//
//At least one example of the target node will be provided, along with some surrounding HTML providing necessary context for you to interpret the meaning and purpose of the HTML target href. When multiple examples of a target node each containing an href are provided, treat the href contained within each target node as being the "same" and generalize as much as possible across its values with respect to the surrounding HTML.
//
//An example of how to perform this task would be to give the name 'profile_url' to an href when all examples of it appear to be a link to the profile page of a user account.
//
//Provide the following information in your response:
//1. name: A generic name in snake case that could be used to represent href values programmatically.
//2. is_page_link: Indicate whether these href(s) likely just point to a new page or if they are for performing some sort of action or mutation. An example of a page link is a link to a another page in a website where more content is consumed such as visiting an 'about' page from the landing page. Action/mutation href likely require a login and change something about the state of an item on the website.
//3. is_peripheral_content: Peripheral content is typically found in headers, footers, sidebars, banners, etc. and does not pertain to the core purpose of the website. Peripheral content is not the primary focus of the website's message or purpose. 
//4. is_advertisement: Indicates if the value is an advertisement.
//5. description: Provide a brief description of this text as if it were a field in a JSON schema
//
//---
//"##);
//    let user_prompt = format!(r##"
//The core purpose of this website has been summarized as follows:
//
//---
//
//{}
//
//---
//
//Use this summary to assist you in determining metadata.
//
//Example(s) of the element node:
//
//---
//
//{}
//
//---
//
//"##, core_purpose, examples);
//    log::debug!("prompt:\n{}{}", system_prompt, user_prompt);
//
//    let response_format = json!({
//        "type": "json_schema",
//        "json_schema": {
//            "name": "href_interpretation_response",
//            "strict": true,
//            "schema": {
//                "type": "object",
//                "properties": {
//                    "name": {
//                        "type": "string"
//                    },
//                    "is_page_link": {
//                        "type": "boolean"
//                    },
//                    "is_peripheral_content": {
//                        "type": "boolean"
//                    },
//                    "is_advertisement": {
//                        "type": "boolean"
//                    },
//                    "description": {
//                        "type": "string"
//                    }
//                },
//                "required": ["name", "is_page_link", "is_peripheral_content", "is_advertisement", "description"],
//                "additionalProperties": false
//            },
//        },
//    });
//
//    let response: LLMElementDataResponse = send_openai_request(system_prompt, user_prompt, response_format).await.expect("Failed to get response from OpenAI");
//
//    NodeData {
//        name: response.name.clone(),
//        element: Some(ElementData {
//            attribute: "href".to_string(),
//            is_page_link: response.is_page_link.clone(),
//            is_peripheral_content: response.is_peripheral_content.clone(),
//            is_advertisement: response.is_advertisement.clone(),
//            description: response.description.clone(),
//        }),
//        text: None,
//    }
//}
