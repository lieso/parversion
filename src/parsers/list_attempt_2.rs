use std::io::{Error, ErrorKind};
use std::io::{self};
use serde_json;

use crate::utilities;
use crate::models;
use crate::prompts;

pub async fn get_list_parser(document: &str) -> Result<Vec<models::list::ListParser>, io::Error> {
    log::trace!("In get_list_parser");

    let mut parsers = Vec::new();

    return Ok(parsers)
}
