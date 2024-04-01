use std::collections::HashMap;
use fancy_regex::{Regex, Captures};
use pandoculation;
use crate::models;

pub fn transform(document: String, parser: &models::curated_listing::CuratedListingParser) -> pandoculation::CuratedListing {
    log::trace!("In transform");

    let regex = Regex::new(&parser.list_pattern).expect("List pattern is not valid");

    let captures: Vec<Captures> = regex
        .captures_iter(&document)
        .filter_map(Result::ok)
        .collect();
    let matches: Vec<&str> = captures
        .iter()
        .filter_map(|cap| cap.get(0).map(|mat| mat.as_str()))
        .collect();
    log::info!("Got {} regex matches for list group", matches.len());

    let list_items = matches.iter().map(|mat| {

        let mut data = pandoculation::CuratedListingItemData {
            title: String::new(),
            url: String::new(),
            author: None,
            id: None,
            points: None,
            timestamp: None,
            chat_url: None,
            additional: HashMap::new(),
        };

        for (key, value) in parser.list_item_patterns.iter() {
            log::debug!("key: {}, value: {}", key, value);

            let regex = Regex::new(value).expect("List item pattern is not valid");

            if let Ok(Some(captures)) = regex.captures(mat) {
                log::debug!("Successfully matched pattern");

                let entire_match = captures.get(0).unwrap().as_str().to_string();
                log::debug!("entire_match: {}", entire_match);

                let first_group = captures.get(1).unwrap().as_str().to_string();
                log::debug!("first_group: {}", first_group);

                match key.as_str() {
                    "title" => data.title = first_group,
                    "url" => data.url = first_group,
                    "author" => data.author = Some(first_group),
                    "id" => data.id = Some(first_group),
                    "points" => data.points = Some(first_group),
                    "timestamp" => data.timestamp = Some(first_group),
                    "chat_url" => data.chat_url = Some(first_group),
                    _ => {
                        data.additional.insert(key.to_string(), first_group);
                    }
                }
            } else {
                log::debug!("Failed to match pattern");
            }
        }

        let list_item = pandoculation::CuratedListingItem {
            data: data,
        };

        return list_item;
    })
    .collect();

    let curated_listing = pandoculation::CuratedListing {
        items: list_items,
    };

    return curated_listing;
}
