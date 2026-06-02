use std::collections::HashMap;
use serde_json::{Value, Map};

use crate::prelude::*;
use crate::data_node::DataNodeFields;

pub struct Json;

impl Json {
    pub fn to_string(map: &Map<String, Value>) -> String {
        map.keys().cloned().collect::<Vec<_>>().join(", ")
    }

    pub fn get_description(map: &Map<String, Value>) -> String {
        map.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
    }

    pub fn get_fields(map: &Map<String, Value>) -> DataNodeFields {
        map.iter()
            .filter_map(|(k, v)| match v {
                Value::Object(_) => None,
                Value::Array(arr) if arr.iter().any(|e| e.is_object()) => None,
                Value::String(s) => Some((k.clone(), s.clone())),
                Value::Number(n) => Some((k.clone(), n.to_string())),
                Value::Bool(b) => Some((k.clone(), b.to_string())),
                Value::Null => Some((k.clone(), "null".to_string())),
                Value::Array(arr) => Some((
                    k.clone(),
                    arr.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(","),
                )),
            })
            .collect()
    }

    pub fn get_children(map: &Map<String, Value>) -> Vec<Map<String, Value>> {
        if map.len() == 1 {
            if let Some((_, Value::Array(arr))) = map.iter().next() {
                return arr
                    .iter()
                    .filter_map(|e| if let Value::Object(m) = e { Some(m.clone()) } else { None })
                    .collect();
            }
        }

        map.iter()
            .flat_map(|(k, v)| match v {
                Value::Object(child_map) => vec![child_map.clone()],
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
        let key_str = keys.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
        Hash::from_str(&format!("object:{}", key_str))
    }
}
