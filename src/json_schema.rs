use serde_json::{json, Value, Map};
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
    original_content: Content,
    target_schema: &String,
    schema_mapping: Vec<SchemaMapping>
) -> Value {
    log::trace!("In apply_schema_mapping");

    fn recurse(
        schema_object: &Map<String, Value>,
        content: &Content,
        array_index: usize,
        source: Vec<String>,
    ) -> Value {
        let object_type = schema_object.get("type")
            .and_then(Value::as_str)
            .expect("Expected `type` to be a string");

        match object_type {
            "object" => {
                let mut object = Value::Object(serde_json::Map::new());
                let object_map = object.as_object_mut().expect("Expected target_json to be an object");

                let properties = schema_object.get("properties")
                    .and_then(Value::as_object)
                    .expect("Expected `properties` to be an object");

                for (key, sub_schema) in properties {
                    let mut new_source = source.clone();
                    new_source.push(key.clone());

                    object_map.insert(
                        key.clone(),
                        recurse(
                            &sub_schema.as_object().expect("Expected property to be an object"),
                            &content,
                            array_index,
                            new_source
                        )
                    );
                }

                object
            },
            "array" => {
                let mut array = Value::Array(Vec::new());
                let array_items = array.as_array_mut().expect("Expected array to be mutable");

                let sub_schema = schema_object.get("items")
                    .and_then(Value::as_object)
                    .expect("Expected `items` to be an object");

                let mut new_array_index: usize = 0;
                let mut new_source = source.clone();
                new_source.push("[]".to_string());

                loop {
                    let maybe_item = recurse(
                        &sub_schema,
                        &content,
                        new_array_index,
                        new_source.clone()
                    );

                    if maybe_item == Value::Null {
                        break;
                    } else {
                        array_items.push(maybe_item);
                        new_array_index += 1;
                    }
                }

                array
            },
            "string" => {

                unimplemented!()

            },
            _ => panic!("Unexpected object type: {}", object_type)
        }
    }

    recurse(
        &serde_json::from_str(target_schema).expect("Invalid JSON"),
        &original_content,
        0,
        Vec::new()
    )
}
