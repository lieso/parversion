use serde_json;
use regex::Regex;

use crate::utilities;
use crate::models;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
}

pub async fn get_list_parser(document: &str) -> Result<Vec<models::list::ListParser>, Errors> {
    log::trace!("In get_list_parser");

    let mut parsers = Vec::new();



    let llm_response = get_patterns(document).await?;
    let json = llm_response.as_object().unwrap();

    let pattern = &json["pattern"];
    log::debug!("pattern: {}", pattern);
    let pattern = serde_json::to_string(pattern).unwrap();
    let pattern = remove_first_and_last(pattern.clone()).unwrap_or(pattern);
    let pattern = &pattern.replace("\\\\", "\\");


    if let Ok(regex) = Regex::new(&pattern) {
        log::debug!("Regex is ok");

        let matches: Vec<&str> = regex
            .captures_iter(document)
            .filter_map(|cap| {
                cap.get(1).map(|mat| mat.as_str())
            })
            .collect();
        log::debug!("{:?}", matches);

        if let Some(first_match) = matches.first() {

            log::debug!("*****************************************************************************************************");
            log::debug!("first_match: {:?}", first_match);

            let llm_response = get_item_patterns(first_match).await?;
            log::debug!("llm_response: {:?}", llm_response);


        } else {
            log::error!("Regex did not result in any matches");
            return Err(Errors::LlmInvalidRegex);
        }
    } else {
        log::error!("Regex is not valid");
        return Err(Errors::LlmInvalidRegex);
    }



    return Ok(parsers)
}

async fn get_item_patterns(document: &str) -> Result<serde_json::Value, Errors> {
    log::trace!("In get_item_patterns");

    let prompt = format!("{} {}", prompts::list::patterns::LIST_ITEM_PROMPT, document);

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

fn remove_first_and_last(s: String) -> Option<String> {
     let chars: Vec<char> = s.chars().collect();
     if chars.len() <= 2 {
         None
     } else {
         Some(chars[1..chars.len() - 1].iter().collect())
     }
 }
