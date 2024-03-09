use serde_json;
use std::collections::HashMap;

use crate::models;
use crate::utilities;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
}

pub async fn adapt_chat_parser(chat_parser: &models::chat::ChatParser) -> Result<models::chat::ChatParser, Errors> {
    log::trace!("In adapt_chat_parser");

    let mut empty_map = HashMap::new();

    for key in chat_parser.chat_item_patterns.keys() {
        empty_map.insert(key.clone(), "");
    }

    let json_string = serde_json::to_string(&empty_map).unwrap();
    log::debug!("json_string: {:?}", json_string);


    let mapping = get_mapping(&json_string).await?;
    log::debug!("mapping: {:?}", mapping);

    panic!("testing");


    return Ok(chat_parser.clone());
}

async fn get_mapping(json: &str) -> Result<serde_json::Map<String, serde_json::Value>, Errors> {
    log::trace!("In get_mapping");

    let prompt = format!("{} {}", prompts::chat::CHAT_ITEM_ADAPTER_PROMPT, json);
    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let json = response
                .as_object()
                .unwrap()
                .clone();

            Ok(json)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
}
