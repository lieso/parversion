use serde_json;
use fancy_regex::Regex;
use std::collections::HashMap;

use crate::utilities;
use crate::models;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
    Unimplemented
}

pub async fn get_parsers(document: &str) -> Result<Vec<models::chat::ChatParser>, Errors> {
    log::trace!("In get_parsers");

    let chat_pattern = get_chat_pattern(document).await?;
    println!("chat_pattern: {:?}", chat_pattern);

    Err(Errors::Unimplemented)
}
    
async fn get_chat_pattern(document: &str) -> Result<String, Errors> {
    log::trace!("In get_chat_pattern");

    let prompt = format!("{} {}", prompts::chat::CHAT_GROUP_PROMPT, document);
    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let json = response
                .as_object()
                .unwrap();
            let pattern = &json["pattern"];
            let pattern = serde_json::to_string(pattern).unwrap();
            let pattern = remove_first_and_last(pattern.clone())
                .unwrap_or(pattern);
            let pattern = &pattern.replace("\\\\", "\\");
            let pattern = pattern.to_string();

            Ok(pattern)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
}

fn remove_first_and_last(s: String) -> Option<String> {
     let chars: Vec<char> = s.chars().collect();
     if chars.len() <= 2 {
         None
     } else {
         Some(chars[1..chars.len() - 1].iter().collect())
     }
 }
