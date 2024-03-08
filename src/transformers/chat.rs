use std::collections::HashMap;
use fancy_regex::Regex;
use crate::models;

pub fn transform(document: String, parser: &models::chat::ChatParser) -> models::chat::Chat {
    log::trace!("In transform");

    let regex = Regex::new(&parser.chat_pattern).expect("List pattern is not valid");

    let matches: Vec<&str> = regex
        .captures_iter(&document)
        .filter_map(|cap| {
            cap.expect("Could not capture")
                .get(0)
                .map(|mat| mat.as_str())
        })
        .collect();
    log::info!("Got {} regex matches for chat group", matches.len());

    let chat_items = matches.iter().map(|mat| {
        let mut chat_item = models::chat::ChatItem {
            data: HashMap::new()
        };

        for (key, value) in parser.chat_item_patterns.iter() {
            log::debug!("key: {}, value: {}", key, value);

            let regex = Regex::new(value).expect("List item pattern is not valid");

            if let Ok(Some(captures)) = regex.captures(mat) {
                log::debug!("Successfully matched pattern");

                let first_match = captures.get(1).unwrap().as_str();
                log::debug!("first_match: {}", first_match);

                chat_item.data.insert(key.to_string(), first_match.to_string());
            } else {
                log::debug!("Failed to match pattern");
            }
        }

        return chat_item;
    })
    .collect();

    let chat = models::chat::Chat {
        items: chat_items,
    };

    return chat;
}
