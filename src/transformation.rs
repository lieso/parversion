use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::{HashMap};
use quick_js::{Context as QuickContext};

use crate::prelude::*;
use crate::id::{ID};
use crate::json_node::{Json, JsonNode, JsonMetadata};
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Runtime {
    AWK,
    NodeJS,
    Python,
    QuickJS,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaTransformation {
    pub id: ID,
    pub description: String,
    pub runtime: Runtime,
    pub source: String,
    pub target: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HashTransformation {
    pub id: ID,
    pub description: String,
    pub runtime: Runtime,
    pub infix: String,
}

impl HashTransformation {
    fn prefix(&self, fields: HashMap<String, String>) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                let fields_js: Vec<String> = fields
                    .into_iter()
                    .map(|(key, value)| {
                        if key == "text" {
                            format!("'{}': '<omitted>'", key)
                        } else {
                            format!("'{}': '{}'", key, value)
                        }
                    })
                    .collect();
                format!("let fields = {{ {} }};", fields_js.join(", "))
            },
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }

    fn suffix(&self) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                format!("JSON.stringify({{ hasherItems }})")
            },
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }

    pub fn transform(
        &self,
        fields: HashMap<String, String>
    ) -> Hash {
        log::trace!("In transform");

        let prefix = self.prefix(fields.clone());
        let suffix = self.suffix();
        let script = format!("{}\n{}\n{}", prefix, self.infix, suffix);

        log::debug!("script: {}", script);

        match self.runtime {
            Runtime::QuickJS => {
                let quick_context = QuickContext::new().unwrap();
                let result = quick_context.eval_as::<String>(&script).unwrap();
                let parsed: Value = serde_json::from_str(&result).unwrap();
                let hasher_items = parsed.get("hasherItems").unwrap();

                if let Some(array) = hasher_items.as_array() {
                    let hasher_items_vec = array
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<String>>();

                    let mut hash = Hash::from_items(hasher_items_vec);
                    hash.finalize();
                    return hash;
                } else {
                    panic!("Expected 'hasherItems' to be an array");
                }
            },
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct XMLElementTransformation {
    pub id: ID,
    pub description: String,
    pub runtime: Runtime,
    pub infix: String,
}

impl XMLElementTransformation {
    fn prefix(&self, element: String, attributes: HashMap<String, String>) -> String {
        let element_code = format!("let element = '{}';", element);

        let attributes_code = {
            let attributes_list: Vec<String> = attributes
                .into_iter()
                .map(|(key, value)| {
                    let escaped_value = serde_json::to_string(&value).unwrap();
                    format!("'{}': {}", key, escaped_value)
                })
                .collect();
            format!("let attributes = {{ {} }};", attributes_list.join(", "))
        };

        format!("{}\n{}", element_code, attributes_code)
    }

    fn suffix(&self) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                format!("JSON.stringify({{ element, attributes }})")
            },
            _ => panic!("unexpected runtime: {:?}", self.runtime),
        }
    }
    
    pub fn transform(
        &self,
        element: String,
        attributes: HashMap<String, String>
    ) -> (
        Option<String>,
        HashMap<String, String>
    ) {
        log::trace!("In transform");

        let prefix = self.prefix(element, attributes);
        let suffix = self.suffix();

        let code = format!("{}\n{}\n{}", prefix, self.infix, suffix);

        match self.runtime {
            Runtime::QuickJS => {
                let quick_context = QuickContext::new().unwrap();

                let result =  quick_context.eval_as::<String>(&code).unwrap();

                let parsed: Value = serde_json::from_str(&result).unwrap();

                let transformed_element = parsed.get("element").and_then(|e|
                    e.as_str().map(String::from));

                let transformed_attributes = parsed.get("attributes")
                    .and_then(|attr| attr.as_object())
                    .map(|attr_obj| {
                        attr_obj.iter().map(|(k, v)| {
                            (k.clone(), v.as_str().unwrap_or("").to_string())
                        }).collect::<HashMap<String, String>>()
                    }).unwrap_or_default();

                (transformed_element, transformed_attributes)
            },
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldMetadata {
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldTransformation {
    pub id: ID,
    pub description: String,
    pub field: String,
    pub image: String,
    pub meta: FieldMetadata,
}

impl FieldTransformation {
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<JsonNode, Errors> {
        log::trace!("In transform");

        if let Some(value) = data_node.fields.get(&self.field) {
            let json = Json {
                key: self.image.clone(),
                value: value.to_string(),
                meta: JsonMetadata {
                    is_primary_content: false,
                },
            };

            let json_node = JsonNode {
                id: ID::new(),
                hash: data_node.hash.clone(),
                lineage: data_node.lineage.clone(),
                description: self.description.clone(),
                parent_id: None,
                json,
            };

            Ok(json_node)
        } else {
            Err(Errors::FieldTransformationFieldNotFound)
        }
    }
}
