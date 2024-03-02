extern crate regex;

use std::collections::HashMap;
use regex::Regex;
use crate::models;

pub fn transform_document_to_list(document: String, parser: &models::list::ListParser) -> models::list::List {
    log::trace!("In transform_document_to_list");

    let regex = Regex::new(&parser.list_pattern).expect("List pattern is not valid");

    let matches: Vec<&str> = regex
        .captures_iter(&document)
        .filter_map(|cap| {
            cap.get(1).map(|mat| mat.as_str())
        })
        .collect();


    let list_items = matches.iter().map(|mat| {

        let mut list_item = models::list::ListItem {
            data: HashMap::new()
        };

        for (key, value) in parser.list_item_patterns.iter() {
            let regex = Regex::new(value).expect("List item pattern is not valid");

            log::debug!("*****************************************************************************************************");
            log::debug!("key: {}", key);
            log::debug!("value: {}", value);
            log::debug!("mat: {}", mat);

            if let Some(captures) = regex.captures(mat) {
                let first_match = captures.get(0).unwrap().as_str();
                log::debug!("first_match: {}", first_match);

                list_item.data.insert(key.to_string(), first_match.to_string());
            }
        }

        return list_item;
    })
    .collect();


    let list = models::list::List {
        items: list_items,
    };

    return list;
}
