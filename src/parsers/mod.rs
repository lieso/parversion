use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::models;
use crate::utilities;
use crate::prompts;

pub async fn get_conversation_parser(document: &str) -> Result<models::ConversationParser, io::Error> {
    log::trace!("In get_conversation_parser");

    let conversation_parser_parent_id = get_conversation_parser_parent_id(document).await.unwrap();
    let conversation_parser_id = get_conversation_parser_id(document).await.unwrap();
    let conversation_parser_content = get_conversation_parser_content(document).await.unwrap();

    let conversation_parser = models::ConversationParser {
        parent_id: conversation_parser_parent_id,
        id: conversation_parser_id,
        content: conversation_parser_content,
    };

    return Ok(conversation_parser)
}

async fn get_conversation_parser_parent_id(document: &str) -> Result<models::ConversationParserParentId, io::Error> {
    log::trace!("In get_conversation_parser_parent_id");

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

            let conversation_parser_parent_id = models::ConversationParserParentId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(conversation_parser_parent_id)
        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
}

async fn get_conversation_parser_id(document: &str) -> Result<models::ConversationParserId, io::Error> {
    log::trace!("In get_conversation_parser_id");
    
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


            let conversation_parser_id = models::ConversationParserId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(conversation_parser_id)

        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }

}

async fn get_conversation_parser_content(document: &str) -> Result<models::ConversationParserContent, io::Error> {
    log::trace!("In get_conversation_parser_content");

    let content = format!("{} {}", prompts::chat::content::PROMPT, document);

    let maybe_open_ai_response = utilities::get_llm_response(content).await;

    match maybe_open_ai_response {
        Ok(prefix_suffix) => {

            let prefix = &prefix_suffix["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);


            let conversation_parser_content = models::ConversationParserContent {
                prefix: prefix.to_string(),
                suffix: suffix.to_string()
            };

            return Ok(conversation_parser_content)

        }
        Err(_e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
}
