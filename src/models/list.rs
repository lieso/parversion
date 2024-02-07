use serde::{Serialize};
use std::collections::HashMap;

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
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

#[derive(Debug)]
#[derive(Serialize)]
pub struct ListItem {
    pub data: HashMap<String, String>,
}

impl ListItem {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

#[derive(Debug)]
#[derive(Serialize)]
pub struct List {
    pub items: Vec<ListItem>,
}
