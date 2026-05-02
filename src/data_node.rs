use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::transformation::HashTransformation;
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
        meaningful_fields: Vec<String>,
        hash_transformation: &HashTransformation,
        fields: DataNodeFields,
        description: String,
        parent_lineage: &Lineage,
    ) -> Self {
        let hash: Hash = hash_transformation.transform(fields.clone());
        let lineage = parent_lineage.with_hash(hash.clone());
        let meaningful_data: DataNodeFields = fields
            .into_iter()
            .filter(|(key, _)| meaningful_fields.contains(key))
            .collect();

        DataNode {
            id: ID::new(),
            hash,
            fields: meaningful_data,
            lineage,
            description,
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
