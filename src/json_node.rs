use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub description: String,
    pub parent_id: Option<String>,
    pub json: Vec<Json>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonMetadata {
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Json {
    pub key: String,
    pub value: String,
    pub schema_type: String,
    pub schema_path: SchemaPath,
    pub schema_description: String,
    pub meta: JsonMetadata,
}
