use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use serde_json::Value;

use crate::prelude::*;
use crate::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    pub id: ID,
    pub name: String,
    pub path: Path,
    #[serde(skip_serializing)]
    pub hash: Hash,
    #[serde(skip_serializing)]
    pub lineage: Lineage,
    //#[serde(skip_serializing)]
    //pub node_lineage: Lineage,
    pub aliases: Vec<String>,
    pub description: String,
    pub data_type: String,
    pub properties: HashMap<String, SchemaNode>,
    pub items: Option<Vec<SchemaNode>>,
}

impl SchemaNode {
    pub fn new(
        name: &str,
        description: &str,
        parent_lineage: &Lineage,
        data_type: &str,
        parent_path: &Path,
    ) -> Self {
        let hash: Hash = Hash::from_str(&name);
        let lineage = parent_lineage.with_hash(hash.clone());

        SchemaNode {
            id: ID::new(),
            hash,
            path: parent_path.clone(),
            lineage,
            name: name.to_string(),
            aliases: Vec::new(),
            description: description.to_string(),
            data_type: data_type.to_string(),
            properties: HashMap::new(),
            items: None,
        }
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
        let path = {
            if is_array {
                parent_path
                    .with_key_segment(name.to_string())
                    .with_any_index_segment()
            } else {
                parent_path
                    .with_key_segment(name.to_string())
            }
        };

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
                        &path,
                        false
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
                    Some(vec![Self::from_serde_value(items_value, name, &lineage, &path, false)?])
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
            path,
            aliases: Vec::new(),
            description: description.to_string(),
            data_type: data_type.to_string(),
            properties,
            items,
        };

        Ok(schema_node)
    }
}
