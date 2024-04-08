use serde_json;
use fancy_regex::Regex;
use std::collections::HashMap;

use crate::utilities;
use crate::models;
use crate::prompts;
use crate::adapters;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
    AdapterError,
    Unimplemented
}

pub async fn get_parsers(document: &str, sample: &str) -> Result<Vec<models::chat::ChatParser>, Errors> {
    log::trace!("In get_parsers");

    let chat_pattern = get_chat_pattern(sample).await?;
    
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
        log::debug!("Got {} matches", matches.len());

        if let Some(_first_match) = matches.first() {
            let sample_matches = matches
                .iter()
                .take(5)
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

    if let Ok(adapted_chat_parser) = adapters::chat::adapt_chat_parser(&chat_parser).await {
        log::debug!("adapted_chat_parser: {:?}", adapted_chat_parser);

        let mut parsers = Vec::new();
        parsers.push(adapted_chat_parser);

        Ok(parsers)
    } else {
        log::error!("Unable to convert chat parser to standard form");
        return Err(Errors::AdapterError);
    }
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
                let pattern = utilities::text::trim_quotes(pattern.clone())
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
            let pattern = utilities::text::trim_quotes(pattern.clone())
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

