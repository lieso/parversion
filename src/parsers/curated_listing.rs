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

pub async fn get_parsers(document: &str, sample: &str) -> Result<Vec<models::curated_listing::CuratedListingParser>, Errors> {
    log::trace!("In get_parsers");

    let list_pattern = get_list_group_pattern(sample).await?;
    log::debug!("list_pattern: {}", list_pattern);

    let mut curated_listing_parser = models::curated_listing::CuratedListingParser::new();
    curated_listing_parser.list_pattern = list_pattern.clone();

    if let Ok(regex) = Regex::new(&list_pattern) {
        log::info!("Regex is ok");

        let captures: Vec<Captures> = regex
            .captures_iter(document)
            .filter_map(Result::ok)
            .collect();

        for (i, cap) in captures.iter().enumerate() {
            let mut all_groups = Vec::new();

            for group_index in 0..cap.len() {
                if let Some(group_match) = cap.get(group_index) {
                    all_groups.push(group_match.as_str());
                }
            }

            log::debug!("Match {}: {:?}", i, all_groups);
        }

        let matches: Vec<&str> = captures
            .iter()
            .filter_map(|cap| cap.get(0).map(|mat| mat.as_str()))
            .collect();

        if let Some(_first_match) = matches.first() {
            let sample_matches = matches
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

            let bad_matches: Vec<_> = matches
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
                get_list_item_patterns(bad_matches).await?
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
    } else {
        log::error!("Regex is not valid");
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

async fn get_list_group_pattern(document: &str) -> Result<String, Errors> {
    log::trace!("In get_list_group_pattern");

    let prompt = format!("{} {}", prompts::curated_listing::CURATED_LISTING_GROUP_PROMPT, document);
    let llm_response = utilities::llm::get_llm_response(prompt).await;

    match llm_response {
        Ok(response) => {
            log::info!("Success response from llm");
            log::debug!("response: {:?}", response);

            let json = response
                .as_object()
                .unwrap();
            let pattern = &json["pattern"];
            let pattern = serde_json::to_string(pattern).unwrap();
            let pattern = remove_first_and_last(pattern.clone())
                .unwrap_or(pattern);
            let pattern = &pattern.replace("\\\\", "\\");
            let pattern = pattern.to_string();

            Ok(pattern)
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
