use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::json_node::{Json, JsonNode};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataNodeFields {
    inner: Vec<(String, String)>,
}

impl DataNodeFields {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.inner.iter()
    }

    pub fn from_hash_map(map: HashMap<String, String>) -> Self {
        Self {
            inner: map.into_iter().collect(),
        }
    }

    pub fn get(&self, key: &str) -> Vec<&String> {
        self.inner
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v)
            .collect()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.iter().any(|(k, _)| k == key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.inner.iter().map(|(k, _)| k)
    }

    pub fn insert(&mut self, key: String, value: String) {
        if !self.inner.iter().any(|(k, v)| *k == key && *v == value) {
            self.inner.push((key, value));
        }
    }

    pub fn get_all(&self, key: &str) -> impl Iterator<Item = &String> {
        let key_str = key.to_string();
        self.inner
            .iter()
            .filter(move |(k, _)| k == &key_str)
            .map(|(_, v)| v)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl IntoIterator for DataNodeFields {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<(String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a DataNodeFields {
    type Item = &'a (String, String);
    type IntoIter = std::slice::Iter<'a, (String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FromIterator<(String, String)> for DataNodeFields {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        let mut fields = Self::new();
        fields.extend(iter);
        fields
    }
}

impl Extend<(String, String)> for DataNodeFields {
    fn extend<T: IntoIterator<Item = (String, String)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub fields: DataNodeFields,
    pub description: String,
}

impl DataNode {
    pub fn new(
        hash: Hash,
        lineage: Lineage,
        fields: DataNodeFields,
        description: String,
    ) -> Self {
        DataNode {
            id: ID::new(),
            hash,
            fields,
            lineage,
            description,
        }
    }

    pub fn from_data_nodes(data_nodes: Vec<Self>) -> Self {
        Self {
            id: ID::new(),
            hash: Hash::new(),
            lineage: Lineage::new(),
            fields: data_nodes.into_iter().fold(
                DataNodeFields::new(),
                |mut acc, data_node| {
                    acc.extend(data_node.fields);
                    acc
                }
            ),
            description: "Placeholder description".to_string()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn to_json_nodes(&self) -> Vec<JsonNode> {
        self.fields
            .iter()
            .map(|(key, value)| {
                let json = Json {
                    key: key.clone(),
                    value: value.clone(),
                };
                JsonNode {
                    id: ID::new(),
                    hash: self.hash.clone(),
                    lineage: self.lineage.clone(),
                    description: self.description.clone(),
                    json,
                }
            })
            .collect()
    }
}
