use std::collections::HashMap;
use fancy_regex::Regex;
use pandoculation;
use crate::models;

pub fn transform(document: String, parser: &models::curated_listing::CuratedListingParser) -> pandoculation::CuratedListing {
    log::trace!("In transform");

    let regex = Regex::new(&parser.list_pattern).expect("List pattern is not valid");

    let matches: Vec<&str> = regex
        .captures_iter(&document)
        .filter_map(|cap| {
            cap.expect("Could not capture")
                .get(1)
                .map(|mat| mat.as_str())
        })
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

                let first_match = captures.get(1).unwrap().as_str().to_string();
                log::debug!("first_match: {}", first_match);

                match key.as_str() {
                    "title" => data.title = first_match,
                    "url" => data.url = first_match,
                    "author" => data.author = Some(first_match),
                    "id" => data.id = Some(first_match),
                    "points" => data.points = Some(first_match),
                    "timestamp" => data.timestamp = Some(first_match),
                    "chat_url" => data.chat_url = Some(first_match),
                    _ => {
                        data.additional.insert(key.to_string(), first_match);
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
