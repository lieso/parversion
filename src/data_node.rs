use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::json_node::{Json, JsonNode};

pub type DataNodeFields = HashMap<String, String>;

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
                HashMap::new(),
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
