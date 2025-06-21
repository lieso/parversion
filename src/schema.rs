use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use serde_json::Value;

use crate::prelude::*;
use crate::transformation::SchemaTransformation;
use crate::provider::Provider;
use crate::schema_node::SchemaNode;

pub type SchemaProperties = HashMap<String, SchemaNode>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub lineage: Lineage,
    pub properties: SchemaProperties,
}

impl Schema {
    pub fn get_contexts(&self) -> Result<(
        HashMap<ID, Arc<SchemaContext>>,
        Graph
    ), Errors> {
        log::trace!("In get_contexts");

        let mut schema_nodes: HashMap<ID, Arc<SchemaNode>> = HashMap::new();
        let mut contexts: HashMap<ID, Arc<Context>> = HashMap::new();

        // dummy node

        unimplemented!()
    }

    pub fn collect_schema_nodes(&self) -> Vec<SchemaNode> {
        log::trace!("In collect_schema_nodes");

        let mut schema_nodes: Vec<SchemaNode> = Vec::new();

        for node in self.properties.values() {
            let mut node_clone = node.clone();
            node_clone.properties.clear();
            node_clone.items = None;

            schema_nodes.push(node_clone);

            node.collect_schema_nodes(&mut schema_nodes);
        }

        schema_nodes
    }

    pub fn from_string(value: &str) -> Result<Self, Errors> {
        if value.trim().is_empty() {
            return Err(Errors::SchemaNotProvided);
        }

        let serde_value: Value = serde_json::from_str(value)
            .expect("Could not parse json schema string");

        let name = serde_value.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Errors::JsonSchemaParseError("Unable to obtain title".to_string()))?;
        log::debug!("name: {}", name);

        let description = serde_value.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Errors::JsonSchemaParseError("Unable to obtain description".to_string()))?;
        log::debug!("description: {}", description);




        //

        let hash = Hash::from_str(&name);
        let lineage = Lineage::from_hashes(vec![hash.clone()]);

        //




        let properties = if let Some(props) = serde_value["properties"].as_object() {
            props
                .iter()
                .map(|(key, val)| {
                    match SchemaNode::from_serde_value(
                        &val,
                        &key,
                        &lineage,
                    ) {
                        Ok(schema_node) => Ok((key.clone(), schema_node)),
                        Err(e) => Err(e),
                    }
                })
                .collect::<Result<HashMap<_, _>, Errors>>()?
        } else {
            HashMap::new()
        };

        let schema = Schema {
            id: ID::new(),
            name: name.to_string(),
            description: description.to_string(),
            lineage,
            properties,
        };

        Ok(schema)
    }

    pub fn to_string_with_target(&self, target_node: &SchemaNode): String -> {
        log::trace!("In to_string_with_target");

        let mut neighbour_ids = HashSet::new();

        unimplemented!()
    }
}

// to minimize context we should incrementally add json one level up till we reach some threshold or include entire schema if threshold is not reached
// that way we maximize information shown to llm about specific objects and not unrelated objects since they will be nested down elsewhere in json tree

pub fn schema_to_string_with_target(
    schema: SchemaProperties,
    target_id: &ID
) -> String {
    log::trace!("In schema_to_string_with_target");

    let entries: Vec<String> = schema
        .iter()
        .map(|(key, node)| {
            if node.id == *target_id {
                format!(r#"START TARGET SCHEMA KEY >>>"{}"<<< END TARGET SCHEMA KEY: {}"#, key, serialize_schema_node(node, target_id))
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
