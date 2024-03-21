use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatParser {
    pub chat_pattern: String,
    pub chat_item_patterns: HashMap<String, String>,
}

impl ChatParser {
    pub fn new() -> Self {
        Self {
            chat_pattern: String::new(),
            chat_item_patterns: HashMap::new(),
        }
    }
}
