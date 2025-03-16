use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub description: String,
    pub parent_id: Option<String>,
    pub json: Json,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonMetadata {
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Json {
    pub key: String,
    pub value: String,
    pub meta: JsonMetadata,
    //pub property: Property,
}
