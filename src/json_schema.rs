use crate::content::{Content};

pub fn content_to_json_schema(content: Content) -> String {
    log::trace!("In content_to_json_schema");

    let json_schema = content.to_json_schema();

    serde_json::to_string(json_schema).expect("Could not serialize JSON schema")
}



pub fn get_schema_mapping(schema_from: String, schema_to: String) -> String {
    unimplemented!()
}
