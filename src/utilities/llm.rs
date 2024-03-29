use std::env;
use std::io::{Error, ErrorKind};
use reqwest::header;
use serde_json::json;
use std::io::{self};

pub async fn get_llm_response(content: String) -> Result<serde_json::Value, io::Error> {
    log::debug!("{}", content);

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
                        log::debug!("{:?}", &choice);

                        let message = &choice["message"];
                        log::debug!("message: {:?}", message);

                        let llm_response = &message["content"].as_str().unwrap();
                        log::debug!("llm_response: {}", llm_response);

                        let llm_response = cleanup_llm_response(llm_response);
                        log::debug!("fixed llm_response: {}", llm_response);

                        // TODO: LLM sometimes does not return json, just the pattern

                        let json: serde_json::Value = serde_json::from_str(&llm_response).expect("Failed to parse json string");
                        log::debug!("{:?}", json);

                        Ok(json)
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

fn cleanup_llm_response(response: &str) -> &str {
    log::trace!("In cleanup_llm_response");

    // remove code-block formatting
    let without_backticks = remove_codeblock_delimiters(response);
    log::debug!("without_backticks: {}", without_backticks);
    let without_label = remove_label_text(without_backticks);
    log::debug!("without_label: {}", without_label);

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
