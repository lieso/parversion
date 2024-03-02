use serde_json;
use fancy_regex::Regex;

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

    let list_pattern = &json["pattern"];
    log::debug!("list_pattern: {}", list_pattern);

    let list_pattern = serde_json::to_string(list_pattern).unwrap();
    let list_pattern = remove_first_and_last(list_pattern.clone()).unwrap_or(list_pattern);
    let list_pattern = &list_pattern.replace("\\\\", "\\");

    let mut list_parser = models::list::ListParser::new();
    list_parser.list_pattern = list_pattern.clone();

    if let Ok(regex) = Regex::new(&list_pattern) {
        log::debug!("Regex is ok");

        let matches: Vec<&str> = regex
            .captures_iter(document)
            .filter_map(|cap| {
                cap.expect("Could not capture").get(1).map(|mat| mat.as_str())
            })
            .collect();
        log::debug!("{:?}", matches);

        if let Some(first_match) = matches.first() {
            let llm_response = get_item_patterns(first_match).await?;
            log::debug!("llm_response: {:?}", llm_response);

            let json = llm_response.as_object().unwrap();

            for (key, pattern) in json {
                log::debug!("key: {}", key);
                log::debug!("value: {}", pattern);

                let pattern = pattern.to_string();
                let pattern = remove_first_and_last(pattern.clone()).unwrap_or(pattern);
                let pattern = &pattern.replace("\\\\", "\\");

                list_parser.list_item_patterns.insert(key.to_string(), pattern.to_string());
            }
        } else {
            log::error!("Regex did not result in any matches");
            return Err(Errors::LlmInvalidRegex);
        }
    } else {
        log::error!("Regex is not valid");
        return Err(Errors::LlmInvalidRegex);
    }

    parsers.push(list_parser);

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
