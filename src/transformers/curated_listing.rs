use std::collections::HashMap;
use fancy_regex::Regex;
use crate::models;

pub fn transform(document: String, parser: &models::curated_listing::CuratedListingParser) -> models::curated_listing::CuratedListing {
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
        let mut list_item = models::curated_listing::CuratedListingItem {
            data: HashMap::new()
        };

        for (key, value) in parser.list_item_patterns.iter() {
            log::debug!("key: {}, value: {}", key, value);

            let regex = Regex::new(value).expect("List item pattern is not valid");

            if let Ok(Some(captures)) = regex.captures(mat) {
                log::debug!("Successfully matched pattern");

                let first_match = captures.get(1).unwrap().as_str();
                log::debug!("first_match: {}", first_match);

                list_item.data.insert(key.to_string(), first_match.to_string());
            } else {
                log::debug!("Failed to match pattern");
            }
        }

        return list_item;
    })
    .collect();

    let curated_listing = models::curated_listing::CuratedListing {
        items: list_items,
    };

    return curated_listing;
}
