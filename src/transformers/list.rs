extern crate regex;

use crate::models;
use regex::Regex;

pub fn transform_document_to_list(document: String, parser: &models::list::ListParser) -> models::list::List {
    log::trace!("In transform_document_to_list");

    for (key, pattern) in parser.patterns.iter() {
        log::debug!("key: {}, pattern: {}", key, pattern);

        if let Ok(regex) = Regex::new(&pattern) {

            let values: Vec<(&str, usize, usize)> = regex
                .captures_iter(&document)
                .filter_map(|cap| {
                    cap.get(1).map(|mat| (mat.as_str(), mat.start(), mat.end()))
                })
                .collect();


            for (value, start, end) in values {
                println!("value: {}, start: {}, end: {}", value, start, end);
            }

        } else {
            log::error!("Regex pattern is invalid");
        }

        let regex = Regex::new(pattern).expect("Failed to parse regex");

        let values: Vec<&str> = regex.find_iter(&document).map(|mat| mat.as_str()).collect();
        println!("{:?}", values);

    }

    let list = models::list::List {
        items: Vec::new(),
    };

    return list;
}
