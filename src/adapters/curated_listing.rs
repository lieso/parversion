use serde_json::{Value};
use std::collections::HashMap;

use crate::models;
use crate::utilities;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
}

pub async fn adapt_curated_listing_parser(curated_listing_parser: &models::curated_listing::CuratedListingParser) -> Result<models::curated_listing::CuratedListingParser, Errors> {
    log::trace!("In adapt_curated_listing_parser");

    let mut adapted_curated_listing_parser = models::curated_listing::CuratedListingParser::new();
    adapted_curated_listing_parser.list_patterns = curated_listing_parser.list_patterns.clone();

    let mut empty_map = HashMap::new();

    for key in curated_listing_parser.list_item_patterns.keys() {
        empty_map.insert(key.clone(), "");
    }

    let json_string = serde_json::to_string(&empty_map).unwrap();
    log::debug!("json_string: {:?}", json_string);

    let mapping = get_mapping(&json_string).await?;
    log::debug!("mapping: {:?}", mapping);

    for key in mapping.keys() {
        log::debug!("key: {}", key);
        
        let old_key = mapping.get(key).unwrap().to_string();
        log::debug!("old_key: {}", old_key);
        let old_key = utilities::text::trim_quotes(old_key.clone()).unwrap_or(old_key);
        log::debug!("old_key: {}", old_key);

        if !old_key.is_empty() {
            let value = curated_listing_parser.list_item_patterns.get(&old_key).unwrap();

            adapted_curated_listing_parser.list_item_patterns.insert(key.to_string(), value.clone());
        }
    }

    for key in curated_listing_parser.list_item_patterns.keys() {
        let mut already_mapped = true;

        for (_, value) in &mapping {
            if let Value::String(str_value) = value {
                if str_value == key {
                    already_mapped = true;
                    break;
                }
            }
        }

        if !already_mapped {
            let value = curated_listing_parser.list_item_patterns.get(key).unwrap();
            adapted_curated_listing_parser.list_item_patterns.insert(key.to_string(), value.to_vec());
        }
    }

    Ok(adapted_curated_listing_parser.clone())
}

async fn get_mapping(json: &str) -> Result<serde_json::Map<String, serde_json::Value>, Errors> {
    log::trace!("In get_mapping");

    let prompt = format!("{} {}", prompts::curated_listing::CURATED_LISTING_ITEM_ADAPTER_PROMPT, json);
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
