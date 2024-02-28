use serde_json;

use crate::utilities;
use crate::models;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
}

pub async fn get_list_parser(document: &str) -> Result<Vec<models::list::ListParser>, Errors> {
    log::trace!("In get_list_parser");

    let mut parsers = Vec::new();



    let llm_response = get_patterns(document).await?;
    println!("{:?}", llm_response);




    return Ok(parsers)
}

async fn get_patterns(document: &str) -> Result<serde_json::Value, Errors> {
    log::trace!("In get_patterns");

    let prompt = format!("{} {}", prompts::list::patterns::LIST_GROUP_PROMPT, document);

    let maybe_llm_response = utilities::llm::get_llm_response(prompt).await;

    match maybe_llm_response {
        Ok(patterns) => {
            log::debug!("Successfully obtained response from llm");
            Ok(patterns)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
}
