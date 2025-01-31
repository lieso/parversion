use serde::{Serialize, Deserialize};
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
    let debug_dir = &read_lock!(CONFIG).dev.debug_dir;
    let db = sled::open(format!("{}/cache", debug_dir)).expect("Could not open cache");
    Arc::new(db)
});

pub struct OpenAI;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponse {
    pub is_unmeaningful: bool,
    pub justification: String,
}

impl OpenAI {
    pub async fn get_field_transformation(
        field: String,
        value: String,
        snippet: String,
    ) -> FieldTransformation {
        log::trace!("In get_field_transformation");

        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");

        log::debug!("field: {:?}", field);
        log::debug!("snippet: {}", snippet);


        if field == "text" {
            let elimination = Self::should_eliminate_text_field(
                value.clone(),
                snippet.clone()
            ).await.expect("Could not determine if text field should be eliminated");
        }


        unimplemented!()
    }

    async fn should_eliminate_text_field(
        value: String,
        snippet: String,
    ) -> Result<bool, Errors> {
        log::trace!("In should_eliminate_text_field");


        let system_prompt = format!(r##"
You interpret the contextual meaning of a specific HTML text node, and infer if the text node represents meaningful natural language meant to be consumed by humans as part of their core purpose in visiting a website, as opposed to ancillary or presentational text.

The specific text node will be contained/delimited with an HTML comment like so:
<!-- Target node: Start -->Text node content here<!-- Target node: End -->

Carefully examine the provided HTML text node along with supplementary information providing crucial context, and determine if any of the following applies to it:

1. If the text node represents an advertisement of some kind.
2. If the text node serves a presentational purpose. For example, a pipe symbol may be used to delineate menu items, other text nodes might represent an icon. Presentational text is not meaningful, semantic content humans consume as part of their core purpose for visiting a website.
3. If the text node is a label for a UI element meant to assist the user in understanding how to operate the website, as opposed to content that is meant to be consumed

Include the following in your response:
1. (is_unmeaningful): if any of the above criteria apply to the text node, respond true
2. (justification): provide justification for your response
        "##);

        let user_prompt = format!(r##"
[Text node]
{}

[Surrounding HTML]
{}
        "##, value.trim(), snippet);

        let response_format = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "elimination_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "is_unmeaningful": {
                            "type": "boolean"
                        },
                        "justification": {
                            "type": "string"
                        }
                    },
                    "required": ["is_unmeaningful", "justification"],
                    "additionalProperties": false
                }
            }
        });

        let response: EliminationResponse = Self::send_openai_request(
            system_prompt.clone(),
            user_prompt.clone(),
            response_format
        ).await.expect("Failed to get response from OpenAI");

        log::debug!("\nn╔════════════════════════════════════════════╗");
        log::debug!("║    SHOULD ELIMINATE TEXT FIELD START       ║");
        log::debug!("╚════════════════════════════════════════════╝");
        
        log::debug!("***system_prompt***\n{}", system_prompt);
        log::debug!("***user_prompt***\n{}", user_prompt);
        log::debug!("***response***\n{:?}", response);

        log::debug!("╔════════════════════════════════════════════╗");
        log::debug!("║    SHOULD ELIMINATE TEXT FIELD END         ║");
        log::debug!("╚════════════════════════════════════════════╝\nn");

        Ok(true)
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
                "model": "gpt-4o",
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
