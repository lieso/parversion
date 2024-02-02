use serde::{Serialize};
use std::collections::HashMap;

#[derive(Debug)]
#[derive(Serialize)]
pub struct ListParser {
    pub patterns: HashMap<String, String>,
}

impl ListParser {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.patterns.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.patterns.get(key)
    }
}

