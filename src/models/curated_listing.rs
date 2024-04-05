use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CuratedListingParser {
    pub list_patterns: Vec<String>,
    pub list_item_patterns: HashMap<String, Vec<String>>,
}

impl CuratedListingParser {
    pub fn new() -> Self {
        Self {
            list_patterns: Vec::new(),
            list_item_patterns: HashMap::new(),
        }
    }
}
