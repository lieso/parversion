use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::models;
use crate::utilities;
use crate::prompts;

pub async fn get_list_parser(document: &str) -> Result<Vec<models::list::ListParser>, io::Error> {
    log::trace!("In get_list_parser");

    let mut parsers = Vec::new();

    let patterns = get_patterns(document).await.unwrap();

    let Some(groups) = patterns.as_array() else {
        log::error!("patterns is not array");
        return Err(Error::new(ErrorKind::InvalidData, "error"));
    };

    for group in groups.iter() {
        let mut list_parser = models::list::ListParser::new();

        let Some(json_object) = group.as_object() else {
            log::error!("Group is not object");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        };

        for (key, value) in json_object {
            log::debug!("Key: {}", key);
            log::debug!("Value: {}", value);

            list_parser.insert(key.to_string(), value.to_string());
        }

        parsers.push(list_parser);
    }

    return Ok(parsers)
}

async fn get_patterns(document: &str) -> Result<serde_json::Value, io::Error> {
    log::trace!("In get_patterns");

    let prompt = format!("{} {}", prompts::list::patterns::PROMPT, document);

    let maybe_llm_response = utilities::get_llm_response(prompt).await;

    match maybe_llm_response {
        Ok(patterns) => {
            return Ok(patterns)
        }
        Err(_e) => {
            log::debug!("Did not receive response from llm");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
}
