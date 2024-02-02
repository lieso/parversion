use std::io::{Error, ErrorKind};
use std::io::{self};

use crate::models;

pub async fn get_list_parser(document: &str) -> Result<models::list::ListParser, io::Error> {
    log::trace!("In get_list_parser");

    let list_parser = models::list::ListParser {
        test: "test".to_string(),
    };

    return Ok(list_parser)
}
