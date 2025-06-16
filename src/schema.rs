use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::transformation::SchemaTransformation;
use crate::provider::Provider;
use crate::schema_node::SchemaNode;

pub type Schema = HashMap<String, SchemaNode>;

pub fn schema_to_string_with_target(
    schema: Schema,
    target_id: &ID
) -> String {
    log::trace!("In schema_to_string_with_target");

    let entries: Vec<String> = schema
        .iter()
        .map(|(key, node)| {
            if node.id == *target_id {
                format!(r#"START TARGET_SCHEMA_KEY >>>"{}"<<< END TARGET SCHEMA KEY: {}"#, key, serialize_schema_node(node, target_id))
            } else {
                format!(r#""{}": {}"#, key, serialize_schema_node(node, target_id))
            }
        })
        .collect();

    format!(r#"{{ {} }}"#, entries.join(", "))
}

fn serialize_schema_node(node: &SchemaNode, target_id: &ID) -> String {
    let properties_json: Vec<String> = node
        .properties
        .iter()
        .map(|(key, value)| {
            if value.id == *target_id {
                format!(r#"START TARGET SCHEMA KEY >>>"{}"<<< END TARGET SCHEMA KEY :{}"#, key, serialize_schema_node(value, target_id))
            } else {
                format!(r#""{}":{}"#, key, serialize_schema_node(value, target_id))
            }
        })
        .collect();

    format!(
        r#"{{
             "description": "{}",
             "data_type": "{}",
             "properties": {{ {} }}
         }}"#,
         node.description,
         node.data_type,
         properties_json.join(", ")
     )
}
