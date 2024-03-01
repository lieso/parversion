use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListParser {
    pub list_pattern: String,
    pub list_item_patterns: HashMap<String, String>,
}

impl ListParser {
    pub fn new() -> Self {
        Self {
            list_pattern: String::new(),
            list_item_patterns: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListItem {
    pub data: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct List {
    pub items: Vec<ListItem>,
}
