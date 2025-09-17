use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use serde_json::Value;

use crate::prelude::*;
use crate::path::{Path, PathSegment};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    pub id: ID,
    pub name: String,
    #[serde(skip_serializing)]
    pub hash: Hash,
    #[serde(skip_serializing)]
    pub lineage: Lineage,
    pub aliases: Vec<String>,
    pub description: String,
    pub data_type: String,
    pub properties: HashMap<String, SchemaNode>,
    pub items: Option<Vec<SchemaNode>>,
    pub path: Path,
}

pub fn arrayify_schema_node(schema_node: &mut SchemaNode, target_path_segment: &ID) {
    schema_node.data_type = "array".to_string();

    fn recurse(
        node: &mut SchemaNode,
        target: &ID
    ) {
        node.path.arrayify(target);

        for child_node in node.properties.values_mut() {
            recurse(
                child_node,
                target,
            );
        }

        if let Some(items) = &mut node.items {
            for item in items {
                recurse(
                    item,
                    target,
                );
            }
        }
    }

    for node in schema_node.properties.values_mut() {
        recurse(
            node,
            target_path_segment,
        );
    }
}

impl SchemaNode {
    pub fn new(
        name: &str,
        description: &str,
        parent_lineage: &Lineage,
        parent_path: &Path,
        data_type: &str,
    ) -> Self {
        let hash: Hash = Hash::from_str(&name);
        let lineage = parent_lineage.with_hash(hash.clone());

        SchemaNode {
            id: ID::new(),
            hash,
            lineage,
            name: name.to_string(),
            aliases: Vec::new(),
            description: description.to_string(),
            data_type: data_type.to_string(),
            properties: HashMap::new(),
            items: None,
            path: parent_path.clone(),
        }
    }

    pub fn get_last_path_segment(&self) -> Option<PathSegment> {
        self.path.segments.last().cloned()
    }
    
    pub fn get_children(&self) -> Vec<SchemaNode> {
        let mut schema_nodes: Vec<SchemaNode> = Vec::new();

        for node in self.properties.values() {
            schema_nodes.push(node.clone());
        }

        if let Some(items) = &self.items {
            for node in items.iter() {
                schema_nodes.push(node.clone());
            }
        }

        schema_nodes
    }

    pub fn from_serde_value(
        value: &Value,
        name: &str,
        parent_lineage: &Lineage,
        parent_path: &Path,
        is_array: bool
    ) -> Result<Self, Errors> {
        log::trace!("In from_serde_value");
        log::debug!("name: {}", name);

        let hash: Hash = Hash::from_str(&name);
        let lineage = parent_lineage.with_hash(hash.clone());
        let path = parent_path.with_key_segment(name.to_string());

        let description = value.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Errors::JsonSchemaParseError("Unable to obtain description".to_string()))?;

        let data_type = value.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Errors::JsonSchemaParseError("Unable to obtain data type".to_string()))?;

        let properties = if let Some(props) = value["properties"].as_object() {
            props
                .iter()
                .map(|(key, val)| {
                    match Self::from_serde_value(
                        &val,
                        &key,
                        &lineage,
                        &parent_path,
                        false,
                    ) {
                        Ok(schema_node) => Ok((key.clone(), schema_node)),
                        Err(e) => Err(e),
                    }
                })
                .collect::<Result<HashMap<_, _>, Errors>>()?
        } else {
            HashMap::new()
        };

        let items = if data_type == "array" {
            if let Some(items_value) = value.get("items") {
                if items_value.is_array() {
                    Some(
                        items_value.as_array().unwrap()
                            .iter()
                            .map(|item_value| Self::from_serde_value(item_value, name, &lineage, &path, true))
                            .collect::<Result<Vec<_>, Errors>>()?
                    )
                } else {

                    let schema_node = Self::from_serde_value(items_value, name, &lineage, &path, false)?;

                    return Ok(schema_node);
                }
            } else {
                None
            }
        } else {
            None
        };

        let schema_node = SchemaNode {
            id: ID::new(),
            hash,
            lineage,
            name: name.to_string(),
            aliases: Vec::new(),
            description: description.to_string(),
            data_type: data_type.to_string(),
            properties,
            items,
            path,
        };

        Ok(schema_node)
    }
}
