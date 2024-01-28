use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::models;
use crate::utilities;
use crate::prompts;

pub async fn get_chat_parser(document: &str) -> Result<models::ChatParser, io::Error> {
    log::trace!("In get_chat_parser");

    let chat_parser_parent_id = get_chat_parser_parent_id(document).await.unwrap();
    let chat_parser_id = get_chat_parser_id(document).await.unwrap();
    let chat_parser_content = get_chat_parser_content(document).await.unwrap();

    let chat_parser = models::ChatParser {
        parent_id: chat_parser_parent_id,
        id: chat_parser_id,
        content: chat_parser_content,
    };

    return Ok(chat_parser)
}

async fn get_chat_parser_parent_id(document: &str) -> Result<models::ChatParserParentId, io::Error> {
    log::trace!("In get_chat_parser_parent_id");

    let content = format!("{} {}", prompts::chat::parent_id::PROMPT, document);

    let maybe_open_ai_response = utilities::get_llm_response(content).await;

    match maybe_open_ai_response {
        Ok(prefix_suffix_relative) => {
            let prefix = &prefix_suffix_relative["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix_relative["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);

            let relative = &prefix_suffix_relative["relative"].as_str().unwrap();
            log::debug!("relative: {}", relative);

            let chat_parser_parent_id = models::ChatParserParentId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(chat_parser_parent_id)
        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
}

async fn get_chat_parser_id(document: &str) -> Result<models::ChatParserId, io::Error> {
    log::trace!("In get_chat_parser_id");
    
    let content = format!("{} {}", prompts::chat::id::PROMPT, document);

    let maybe_open_ai_response = utilities::get_llm_response(content).await;

    match maybe_open_ai_response {
        Ok(prefix_suffix_relative) => {

            let prefix = &prefix_suffix_relative["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix_relative["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);

            let relative = &prefix_suffix_relative["relative"].as_str().unwrap();
            log::debug!("relative: {}", relative);


            let chat_parser_id = models::ChatParserId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(chat_parser_id)

        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }

}

async fn get_chat_parser_content(document: &str) -> Result<models::ChatParserContent, io::Error> {
    log::trace!("In get_chat_parser_content");

    let content = format!("{} {}", prompts::chat::content::PROMPT, document);

    let maybe_open_ai_response = utilities::get_llm_response(content).await;

    match maybe_open_ai_response {
        Ok(prefix_suffix) => {

            let prefix = &prefix_suffix["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);


            let chat_parser_content = models::ChatParserContent {
                prefix: prefix.to_string(),
                suffix: suffix.to_string()
            };

            return Ok(chat_parser_content)

        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
}
