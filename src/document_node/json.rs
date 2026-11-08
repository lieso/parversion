use serde_json::{Map, Value};

use crate::data_node::{DataNodeFields, FieldValue};
use crate::prelude::*;

pub struct Json;

impl Json {
    pub fn get_name(map: &Map<String, Value>) -> String {
        if map.len() == 1 {
            if let Some((key, val)) = map.iter().next() {
                match val {
                    Value::Object(_) => return key.clone(),
                    Value::Array(arr) if arr.iter().any(|e| e.is_object()) => return key.clone(),
                    _ => {}
                }
            }
        }
        String::new()
    }

    pub fn to_string(map: &Map<String, Value>) -> String {
        map.keys().cloned().collect::<Vec<_>>().join(", ")
    }

    pub fn get_description(map: &Map<String, Value>) -> String {
        map.keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn get_fields(map: &Map<String, Value>) -> DataNodeFields {
        map.iter()
            .flat_map(|(k, v)| -> Vec<(String, FieldValue)> {
                match v {
                    Value::Object(_) => vec![],
                    Value::Array(arr) if arr.iter().any(|e| e.is_object()) => vec![],
                    Value::Array(arr) => arr
                        .iter()
                        .map(|e| (k.clone(), Self::serde_value_to_field_value(e)))
                        .collect(),
                    scalar => vec![(k.clone(), Self::serde_value_to_field_value(scalar))],
                }
            })
            .collect()
    }

    fn serde_value_to_field_value(value: &Value) -> FieldValue {
        match value {
            Value::String(s) => FieldValue::Text(s.clone()),
            Value::Number(n) => match n.as_f64() {
                Some(f) => FieldValue::Float(f),
                None => FieldValue::Text(n.to_string()),
            },
            Value::Bool(b) => FieldValue::Bool(b.clone()),
            Value::Null => FieldValue::Text("null".to_string()),
            other => FieldValue::Text(other.to_string()),
        }
    }

    pub fn get_attribute_value(map: &Map<String, Value>, attribute: &str) -> Option<String> {
        match map.get(attribute)? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Null => Some("null".to_string()),
            Value::Object(_) => None,
            Value::Array(arr) if arr.iter().any(|e| e.is_object()) => None,
            Value::Array(arr) => Some(
                arr.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        }
    }

    pub fn get_children(map: &Map<String, Value>) -> Vec<Map<String, Value>> {
        if map.len() == 1 {
            if let Some((_, Value::Array(arr))) = map.iter().next() {
                return arr
                    .iter()
                    .filter_map(|e| {
                        if let Value::Object(m) = e {
                            Some(m.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
            }

            if let Some((_, Value::Object(inner))) = map.iter().next() {
                return vec![inner.clone()];
            }
        }

        map.iter()
            .flat_map(|(k, v)| match v {
                Value::Object(_) => {
                    let mut wrapper = Map::new();
                    wrapper.insert(k.clone(), v.clone());
                    vec![wrapper]
                }
                Value::Array(arr) if arr.iter().any(|e| e.is_object()) => {
                    let mut wrapper = Map::new();
                    wrapper.insert(k.clone(), Value::Array(arr.clone()));
                    vec![wrapper]
                }
                _ => vec![],
            })
            .collect()
    }

    pub fn get_hash(map: &Map<String, Value>) -> Hash {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        let key_str = keys
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Hash::from_str(&format!("object:{}", key_str))
    }
}
