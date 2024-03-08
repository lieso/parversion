use std::collections::HashMap;
use crate::models;

pub async fn adapt_chat_parser(chat_parser: &models::chat::ChatParser) -> models::chat::ChatParser) -> {
    log::trace!("In adapt_chat_parser");

    let mut empty_map = HashMap::new();

    for key in chat_parser.keys() {
        empty_map.insert(*key, "");
    }

    let json_string = serde_json::to_string(&empty_map).unwrap();


}
