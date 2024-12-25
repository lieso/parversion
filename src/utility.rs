use std::io::{Read};
use std::fs::File;

use crate::types::*;

pub fn get_file_as_string(path: String) -> Result<String, Errors> {
    let mut text = String::new();

    let mut file = File::open(path).map_err(|err| {
        log::error!("Failed to open file: {}", err);
        Errors::FileInputError
    })?;

    file.read_to_string(&mut text).map_err(|err| {
        log::error!("Failed to read file: {}", err);
        Errors::FileInputError
    })?;

    text
}
