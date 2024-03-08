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
    
    let mut chat_parser = models::chat::ChatParser::new();
    chat_parser.chat_pattern = chat_pattern.clone();

    if let Ok(regex) = Regex::new(&chat_pattern) {
        log::info!("Regex is ok");

        let matches: Vec<&str> = regex
            .captures_iter(document)
            .filter_map(|cap| {
                cap.expect("Could not capture").get(0).map(|mat| mat.as_str())
            })
            .collect();
        log::debug!("{:?}", matches);

        if let Some(_first_match) = matches.first() {
            let sample_matches = matches
                .iter()
                .take(3)
                .cloned()
                .collect();

            chat_parser.chat_item_patterns = get_chat_item_patterns(sample_matches).await?;
        } else {
            log::error!("Regex did not result in any matches");
            return Err(Errors::LlmInvalidRegex);
        }
    } else {
        log::error!("Regex did not result in any matches");
        return Err(Errors::LlmInvalidRegex);
    }

    let mut parsers = Vec::new();
    parsers.push(chat_parser);

    Ok(parsers)
}

async fn get_chat_item_patterns(samples: Vec<&str>) -> Result<HashMap<String, String>, Errors> {
    log::trace!("In get_chat_item_patterns");

    let mut prompt = format!("{}", prompts::chat::CHAT_ITEM_PROMPT);

    for (index, &item) in samples.iter().enumerate() {
        prompt = format!("{}\nExample {}\n{}", prompt, index + 1, item);
    }

    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let mut patterns = HashMap::new();

            let json = response
                .as_object()
                .unwrap();
            for (key, pattern) in json {
                log::debug!("key: {}, pattern: {}", key, pattern);

                let pattern = pattern.to_string();
                let pattern = remove_first_and_last(pattern.clone())
                    .unwrap_or(pattern);
                let pattern = &pattern.replace("\\\\", "\\");

                patterns.insert(key.to_string(), pattern.to_string());
            }

            Ok(patterns)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
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
