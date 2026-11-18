use serde_json;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use url::Url;

use crate::types::*;

pub fn get_file_as_text(path: &str) -> Result<String, Errors> {
    let mut text = String::new();

    let mut file = File::open(path).map_err(|err| {
        log::error!("Failed to open file: {}", err);
        Errors::FileInputError
    })?;

    file.read_to_string(&mut text).map_err(|err| {
        log::error!("Failed to read file: {}", err);
        Errors::FileInputError
    })?;

    Ok(text)
}

pub fn write_text_to_file(path: &str, text: &str) -> io::Result<()> {
    let mut file = File::create(path)?;

    file.write_all(text.as_bytes())?;

    Ok(())
}

pub fn is_valid_url(string: &str) -> bool {
    match Url::parse(string) {
        Ok(url) => {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                return false;
            }

            if url.host_str().is_none() || url.host_str().unwrap().is_empty() {
                return false;
            }

            true
        }
        Err(_) => false,
    }
}

pub fn shorten_url(url_string: &str) -> String {
    const MAX_PARAM_LENGTH: usize = 30;

    let mut url = Url::parse(url_string).expect("Invalid URL");

    let query_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| {
            let shortened_value = if value.len() > MAX_PARAM_LENGTH {
                format!("{}...", &value[..MAX_PARAM_LENGTH])
            } else {
                value.to_string()
            };
            (key.to_string(), shortened_value)
        })
        .collect();

    if !query_pairs.is_empty() {
        url.query_pairs_mut().clear();
        for (key, value) in query_pairs {
            url.query_pairs_mut().append_pair(&key, &value);
        }
    }

    url.to_string()
}

pub fn is_valid_unix_path(string: &str) -> bool {
    Path::new(string).exists()
}

pub fn is_valid_json(string: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(string).is_ok()
}
