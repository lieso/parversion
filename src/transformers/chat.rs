use std::collections::HashMap;
use fancy_regex::Regex;
use crate::models;
use pandoculation;
use html_escape;

pub fn transform(document: String, parser: &models::chat::ChatParser) -> pandoculation::Chat {
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
        let mut data = pandoculation::ChatItemData {
            text: String::new(),
            author: String::new(),
            id: String::new(),
            parent_id: None,
            timestamp: None,
            additional: HashMap::new(),
        };

        for (key, value) in parser.chat_item_patterns.iter() {
            log::debug!("key: {}, value: {}", key, value);
            let regex = Regex::new(value).expect("List item pattern is not valid");

            if let Ok(Some(captures)) = regex.captures(mat) {
                log::debug!("Successfully matched pattern");

                match captures.get(1) {
                    Some(m) => {
                        let first_match = m.as_str().to_string();
                        log::debug!("first_match: {}", first_match);

                        // TODO: does this belong here?
                        let value = html_escape::decode_html_entities(&first_match).into_owned();

                        match key.as_str() {
                            "text" => data.text = value,
                            "author" => data.author = value,
                            "id" => data.id = value,
                            "parent_id" => data.parent_id = Some(value),
                            "timestamp" => data.timestamp = Some(value),
                            _ => {
                                data.additional.insert(key.to_string(), value);
                            }
                        }
                    },
                    None => {
                        log::warn!("Expected capture group not found in regex match");
                    }
                }
            } else {
                log::debug!("Failed to match pattern");
            }
        }

        let chat_item = pandoculation::ChatItem {
            data: data,
        };

        return chat_item;
    })
    .collect();

    let chat = pandoculation::Chat {
        items: chat_items,
    };

    return chat;
}
