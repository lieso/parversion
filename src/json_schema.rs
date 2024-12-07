use crate::content::{Content};
use crate::llm;

pub struct SchemaMapping {
    pub source: String,
    pub target: String,
}

pub fn content_to_json_schema(content: Content) -> String {
    log::trace!("In content_to_json_schema");

    let json_schema = content.clone().to_json_schema();

    serde_json::to_string(&json_schema).expect("Could not serialize JSON schema")
}



pub async fn get_schema_mapping(schema_from: &String, schema_to: &String) -> Vec<SchemaMapping> {
    let schema_mapping = llm::get_schema_mapping(schema_from.clone(), schema_to.clone()).await;

    schema_mapping.mappings.iter().map(|item| {
        SchemaMapping {
            source: item.source.clone(),
            target: item.target.clone(),
        }
    }).collect()
}

pub fn apply_schema_mapping(
    content: Content,
    target_schema: &String,
    schema_mapping: Vec<SchemaMapping>
) -> Content {
    log::trace!("In apply_schema_mapping");

    unimplemented!()
}
