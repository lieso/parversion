use std::env;
use std::io::{Error, ErrorKind};
use reqwest::header;
use serde_json::json;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::io::{self};

const LLM_RESPONSE_STORE_PATH: &str = "src/data/llm_response_store";

pub async fn get_llm_response(content: String) -> Result<serde_json::Value, io::Error> {
    log::trace!("In get_llm_response");

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_result = hasher.finalize();
    let hash = hash_result.iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    log::debug!("hash: {}", hash);

    if cfg!(debug_assertions) {
        log::info!("Running in debug mode");

        let llm_response_store = get_llm_response_store().unwrap();

        if let Some(response) = llm_response_store.get(&hash) {
            return Ok(cleanup_llm_response(response));
        }
    } else {
        log::info!("Running in release mode");
    }

    if let Ok(openai_api_key) = env::var("OPENAI_API_KEY") {
        let request_json = json!({
            "model":  "gpt-4-0125-preview",
            "temperature":  0,
            "messages":  [
                {
                    "role": "user",
                    "content": content
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

        match response {
            Ok(success_response) => {
                let json_response = success_response.json::<serde_json::Value>().await;
                match json_response {
                    Ok(json_data) => {
                        log::debug!("LLM json response: {:?}", json_data);

                        let Some(choices) = json_data["choices"].as_array() else {
                            log::error!("Could not get choices array from OpenAI response");
                            return Err(Error::new(ErrorKind::InvalidData, "error"));
                        };

                        let choice = &choices[0];
                        log::trace!("{:?}", &choice);

                        let message = &choice["message"];
                        log::trace!("message: {:?}", message);

                        let llm_response = &message["content"].as_str().unwrap();
                        log::trace!("llm_response: {}", llm_response);

                        if cfg!(debug_assertions) {
                            log::info!("Saving LLM response to store");

                            let mut llm_response_store = get_llm_response_store().unwrap();
                            llm_response_store.insert(hash.to_string(), llm_response.to_string());

                            save_llm_response_store(&llm_response_store).unwrap();
                        }

                        Ok(cleanup_llm_response(llm_response))
                    }
                    Err(_err) => {
                        return Err(Error::new(ErrorKind::InvalidData, "error"));
                    }
                }
            },
            Err(_err) => {
                return Err(Error::new(ErrorKind::InvalidData, "error"));
            }
        }
    } else {
        log::error!("OPENAI_API_KEY could not be found in environment!");
        return Err(Error::new(ErrorKind::InvalidData, "error"));
    }
}

fn cleanup_llm_response(response: &str) -> serde_json::Value {
    log::trace!("In cleanup_llm_response");

    // remove code-block formatting
    let without_backticks = remove_codeblock_delimiters(response);
    log::trace!("without_backticks: {}", without_backticks);

    let without_label = remove_label_text(without_backticks);
    log::trace!("without_label: {}", without_label);

    if let Ok(json) = serde_json::from_str(&without_label) {
        return json;
    } else {
        return serde_json::Value::String(without_label.to_string());
    }
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

fn get_llm_response_store() -> Result<HashMap<String, String>, serde_json::Error> {
   match fs::read_to_string(&LLM_RESPONSE_STORE_PATH) {
       Ok(contents) => {
           serde_json::from_str(&contents)
       },
       Err(_) => {
           Ok(HashMap::new())
       }
   }
}

fn save_llm_response_store(map: &HashMap<String, String>) -> Result<(), serde_json::Error> {
    let json_contents = serde_json::to_string_pretty(map)?;
    fs::write(LLM_RESPONSE_STORE_PATH, json_contents).unwrap();
    Ok(())
}
