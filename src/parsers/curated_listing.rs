use serde_json;
use fancy_regex::Regex;

use crate::utilities;
use crate::models;
use crate::prompts;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
}

pub async fn get_parsers(document: &str) -> Result<Vec<models::curated_listing::CuratedListingParser>, Errors> {
    log::trace!("In get_parsers");

    let mut parsers = Vec::new();


    Ok(parsers)
}
