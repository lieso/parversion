use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub description: String,
    pub json: Json,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Json {
    pub key: String,
    pub value: Value,
}
