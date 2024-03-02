use std::collections::HashMap;
use fancy_regex::Regex;
use crate::models;

pub fn transform_document_to_list(document: String, parser: &models::list::ListParser) -> models::list::List {
    log::trace!("In transform_document_to_list");

    let regex = Regex::new(&parser.list_pattern).expect("List pattern is not valid");

    let matches: Vec<&str> = regex
        .captures_iter(&document)
        .filter_map(|cap| {
            cap.expect("Could not capture").get(1).map(|mat| mat.as_str())
        })
        .collect();


    let list_items = matches.iter().map(|mat| {

        let mut list_item = models::list::ListItem {
            data: HashMap::new()
        };

        for (key, value) in parser.list_item_patterns.iter() {
            let regex = Regex::new(value).expect("List item pattern is not valid");

            if let Ok(captures) = regex.captures(mat) {
                if let Some(captures) = captures {

                    for (i, capture) in captures.iter().enumerate() {
                        match capture {
                            Some(m) => {
                                log::debug!("{}: {}", i, m.as_str());
                            }
                            None => {
                                log::debug!("{}: No match", i);
                            }
                        }
                    }

                    let first_match = captures.get(1).unwrap().as_str();
                    log::debug!("first_match: {}", first_match);

                    list_item.data.insert(key.to_string(), first_match.to_string());
                }
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
