use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::models;
use crate::utilities;
use crate::prompts;

pub async fn get_list_parser(document: &str) -> Result<models::list::ListParser, io::Error> {
    log::trace!("In get_list_parser");

    let patterns = get_patterns(document).await.unwrap();
    println!("{:?}", patterns);

    let list_parser = models::list::ListParser {
        test: "test".to_string(),
    };

    return Ok(list_parser)
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
