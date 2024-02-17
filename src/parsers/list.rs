use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::utilities;
use crate::models;
use crate::prompts;

pub async fn get_list_parser(document: &str) -> Result<Vec<models::list::ListParser>, io::Error> {
    log::trace!("In get_list_parser");

    let mut parsers = Vec::new();

    let llm_response = get_patterns(document).await.unwrap();

    let Some(groups) = llm_response.as_array() else {
        log::error!("patterns is not array");
        return Err(Error::new(ErrorKind::InvalidData, "error"));
    };

    for group in groups.iter() {
        let mut list_parser = models::list::ListParser::new();

        let Some(json_object) = group.as_object() else {
            log::error!("Group is not object");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        };





        //let example_original_list_item = serde_json::to_string(&json_object["example"]).unwrap();

        let chat_ref_llm_response = get_chat_ref(document).await.unwrap();
        log::debug!("{:?}", chat_ref_llm_response);

        let Some(chat_object) = chat_ref_llm_response.as_object() else {
            log::error!("Chat ref is not object");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        };

        let chat_ref_pattern = serde_json::to_string(&chat_object["chat"]).unwrap();
        log::debug!("chat_ref_pattern: {}", chat_ref_pattern);

        match remove_first_and_last(chat_ref_pattern.to_string()) {
            Some(fixed_value) => {
                list_parser.insert("_chat".to_string(), fixed_value);
            }
            None => {
                log::debug!("string less than two characters");
            }
        }







        let Some(patterns_object) = group["patterns"].as_object() else {
            log::error!("Patterns is not object");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        };

        for (key, value) in patterns_object {
            log::debug!("Key: {}", key);
            log::debug!("Value: {}", value);

            match remove_first_and_last(value.to_string()) {
                Some(fixed_value) => {
                    list_parser.insert(key.to_string(), fixed_value);
                }
                None => {
                    log::debug!("string less than two characters");
                }
            }
        }

        parsers.push(list_parser);
    }

    return Ok(parsers)
}

fn remove_first_and_last(s: String) -> Option<String> {
     let chars: Vec<char> = s.chars().collect();
     if chars.len() <= 2 {
         None
     } else {
         Some(chars[1..chars.len() - 1].iter().collect())
     }
 }

async fn get_patterns(document: &str) -> Result<serde_json::Value, io::Error> {
    log::trace!("In get_patterns");

    let prompt = format!("{} {}", prompts::list::patterns::PROMPT, document);

    let maybe_llm_response = utilities::llm::get_llm_response(prompt).await;

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

async fn get_chat_ref(document: &str) -> Result<serde_json::Value, io::Error> {
    log::trace!("In get_chat_ref");

    let prompt = format!("{} {}", prompts::list::patterns::CHAT_REF_PROMPT, document);

    let maybe_llm_response = utilities::llm::get_llm_response(prompt).await;

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
