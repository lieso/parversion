use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CuratedListingParser {
    pub list_pattern: String,
    pub list_item_patterns: HashMap<String, Vec<String>>,
}

impl CuratedListingParser {
    pub fn new() -> Self {
        Self {
            list_pattern: String::new(),
            list_item_patterns: HashMap::new(),
        }
    }
}
