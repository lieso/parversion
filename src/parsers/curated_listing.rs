use serde_json;
use fancy_regex::{Regex, Captures};
use std::collections::HashMap;

use crate::utilities;
use crate::models;
use crate::prompts;
use crate::adapters;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
    AdapterError,
}

const SELF_IMPROVE_ATTEMPTS: u8 = 2;

pub async fn get_parsers(document: &str, sample: &str) -> Result<Vec<models::curated_listing::CuratedListingParser>, Errors> {
    log::trace!("In get_parsers");

    let list_patterns = get_list_group_patterns(sample).await?;
    log::debug!("list_patterns: {:?}", list_patterns);

    let mut curated_listing_parser = models::curated_listing::CuratedListingParser::new();



    let mut all_matches = Vec::new();

    for list_pattern in list_patterns.iter() {

        let mut self_improve_attempt = 0;
        let mut current_list_pattern = list_pattern.clone();

        while self_improve_attempt < SELF_IMPROVE_ATTEMPTS {
            log::debug!("self_improve_attempt no: {}", self_improve_attempt);
            log::debug!("current_list_pattern: {}", current_list_pattern);

            match Regex::new(&current_list_pattern) {
                Ok(regex) => {
                    log::info!("Regex is ok");

                    let captures: Vec<Captures> = regex
                        .captures_iter(document)
                        .filter_map(Result::ok)
                        .collect();

                    let matches: Vec<&str> = captures
                        .iter()
                        .filter_map(|cap| cap.get(0).map(|mat| mat.as_str()))
                        .collect();

                    if matches.is_empty() {
                        log::error!("Regular expression did not result in any matches!");

                        current_list_pattern = self_improve_list_pattern(&sample, &current_list_pattern).await?;
                        self_improve_attempt += 1;
                    } else {
                        log::info!("Regular expression resulted in matches");
                        curated_listing_parser.list_patterns.push(current_list_pattern);
                        all_matches.extend(matches);
                        break;
                    }
                }
                Err(_) => {
                    log::error!("Regex `{}` is not valid", current_list_pattern);
                    return Err(Errors::LlmInvalidRegex);
                }
            }

        }
    }


    if curated_listing_parser.list_patterns.is_empty() {
        log::error!("LLM was unable to generate matching regular expressions");
        return Err(Errors::LlmInvalidRegex);
    }




    if let Some(_first_match) = all_matches.first() {
        let sample_matches = all_matches
            .iter()
            .take(3)
            .cloned()
            .collect();

        let list_item_patterns = get_list_item_patterns(sample_matches).await?;




        let regexes: Result<Vec<_>, _> = list_item_patterns
            .values()
            .map(|v| Regex::new(v))
            .collect();

        let regexes = regexes.expect("Some list item patterns are not valid regexes");

        let bad_matches: Vec<_> = all_matches
            .iter()
            .flat_map(|mat| {
                regexes.iter().filter_map(move |regex| {
                    if let Ok(Some(_captures)) = regex.captures(mat) {
                        None
                    } else {
                        log::debug!("Failed to match pattern; identified bad match");
                        Some(*mat)
                    }
                })
            })
            .collect();

        log::debug!("bad_matches: {:?}", bad_matches);
        log::debug!("Found {} bad matches", bad_matches.len());

        let second_round_patterns = if !bad_matches.is_empty() {
            let sample_bad_matches = bad_matches
                .iter()
                .take(3)
                .cloned()
                .collect();
            get_list_item_patterns(sample_bad_matches).await?
        } else {
            HashMap::new()
        };
        log::debug!("second_round_patterns: {:?}", second_round_patterns);

        let mut merged_patterns: HashMap<String, Vec<String>> = HashMap::new();

        for (key, value) in list_item_patterns {
            merged_patterns.entry(key).or_insert_with(Vec::new).push(value);
        }

        for (key, value) in second_round_patterns {
            merged_patterns.entry(key).or_insert_with(Vec::new).push(value);
        }

        log::debug!("merged_patterns: {:?}", merged_patterns);


        curated_listing_parser.list_item_patterns = merged_patterns;



    } else {
        log::error!("Regex did not result in any matches");
        return Err(Errors::LlmInvalidRegex);
    }









    if let Ok(adapted_curated_listing_parser) = adapters::curated_listing::adapt_curated_listing_parser(&curated_listing_parser).await {
        log::debug!("adapted_curated_listing_parser: {:?}", adapted_curated_listing_parser);

        let mut parsers = Vec::new();
        parsers.push(adapted_curated_listing_parser);

        Ok(parsers)
    } else {
        log::error!("Unable to convert curated listing parser to standard form");
        return Err(Errors::AdapterError);
    }
}

async fn self_improve_list_pattern(sample: &str, bad_pattern: &str) -> Result<String, Errors> {
    log::trace!("In self_improve_list_pattern");

    let prompt = prompts::curated_listing::self_improve_list_group_pattern(bad_pattern, sample);

    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let improved_pattern = serde_json::to_string(&response).unwrap();
            log::debug!("improved_pattern: {}", improved_pattern);
            let improved_pattern = utilities::text::trim_quotes(improved_pattern.clone())
                .unwrap_or(improved_pattern);
            log::debug!("improved_pattern: {}", improved_pattern);
            let improved_pattern = &improved_pattern.replace("\\\\", "\\");
            log::debug!("improved_pattern: {}", improved_pattern);
            let improved_pattern = improved_pattern.to_string();
            log::debug!("improved_pattern: {}", improved_pattern);

            Ok(improved_pattern)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
}

async fn get_list_item_patterns(samples: Vec<&str>) -> Result<HashMap<String, String>, Errors> {
    log::trace!("In get_list_item_patterns");

    let mut prompt = format!("{}", prompts::curated_listing::CURATED_LISTING_ITEM_PROMPT);

    for (index, &item) in samples.iter().enumerate() {
        prompt = format!("{}\nExample {}\n{}", prompt, index + 1, item);
    }

    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let mut patterns = HashMap::new();

            let json = response
                .as_object()
                .unwrap();
            for (key, pattern) in json {
                log::debug!("key: {}, pattern: {}", key, pattern);

                let pattern = pattern.to_string();
                log::debug!("pattern: {}", pattern);
                let pattern = remove_first_and_last(pattern.clone())
                    .unwrap_or(pattern);
                log::debug!("pattern: {}", pattern);
                let pattern = &pattern.replace("\\\\", "\\");
                log::debug!("pattern: {}", pattern);

                patterns.insert(key.to_string(), pattern.to_string());
            }

            Ok(patterns)
        }
        Err(error) => {
            log::error!("{}", error);
            Err(Errors::LlmRequestError)
        }
    }
}

async fn get_list_group_patterns(document: &str) -> Result<Vec<String>, Errors> {
    log::trace!("In get_list_group_patterns");

    let prompt = format!("{} {}", prompts::curated_listing::CURATED_LISTING_GROUP_PROMPT, document);
    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let json = response
                .as_array()
                .unwrap();

            let mut patterns = Vec::new();

            for pattern in json.iter() {
                log::debug!("pattern: {}", pattern);

                let pattern = serde_json::to_string(pattern).unwrap();
                log::debug!("pattern: {}", pattern);
                let pattern = utilities::text::trim_quotes(pattern.clone())
                    .unwrap_or(pattern);
                log::debug!("pattern: {}", pattern);
                let pattern = &pattern.replace("\\\\", "\\");
                log::debug!("pattern: {}", pattern);
                let pattern = pattern.to_string();
                log::debug!("pattern: {}", pattern);

                patterns.push(pattern);
            }

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
