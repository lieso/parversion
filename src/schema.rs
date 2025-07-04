use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use serde_json::Value;

use crate::prelude::*;
use crate::transformation::SchemaTransformation;
use crate::provider::Provider;
use crate::schema_node::SchemaNode;
use crate::schema_context::SchemaContext;
use crate::graph_node::{GraphNode, Graph};
use crate::path::Path;

pub type SchemaProperties = HashMap<String, SchemaNode>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    pub id: ID,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing)]
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
        let mut schema_contexts: HashMap<ID, Arc<SchemaContext>> = HashMap::new();

        let dummy_node = SchemaNode {
            id: self.id.clone(),
            name: self.name.clone(),
            path: Path::new(),
            description: self.description.clone(),
            hash: Hash::from_str(&self.name),
            lineage: self.lineage.clone(),
            properties: self.properties.clone(),
            items: None,
            aliases: Vec::new(),
            data_type: "object".to_string(),
        };

        fn recurse(
            current: Arc<SchemaNode>,
            parents: Vec<Graph>,
            schema_nodes: &mut HashMap<ID, Arc<SchemaNode>>,
            contexts: &mut HashMap<ID, Arc<SchemaContext>>,
        ) -> Graph {
            schema_nodes.insert(current.id.clone(), Arc::clone(&current));

            let graph_node = Arc::new(RwLock::new(
                GraphNode::from_schema_node(
                    Arc::clone(&current),
                    parents.clone(),
                )
            ));

            let schema_context = Arc::new(SchemaContext {
                id: ID::new(),
                lineage: current.lineage.clone(),
                schema_node: Arc::clone(&current),
                graph_node: Arc::clone(&graph_node),
            });

            contexts.insert(current.id.clone(), Arc::clone(&schema_context));
            contexts.insert(read_lock!(graph_node).id.clone(), Arc::clone(&schema_context));

            {
                let children: Vec<Graph> = current
                    .get_children()
                    .into_iter()
                    .map(|child| {
                        recurse(
                            Arc::new(child),
                            vec![Arc::clone(&graph_node)],
                            schema_nodes,
                            contexts,
                        )
                    })
                    .collect();

                let mut write_lock = write_lock!(graph_node);

                let child_hashes: Vec<Hash> = children.iter()
                    .map(|child| read_lock!(child).hash.clone())
                    .collect();

                let mut subgraph_hash = Hash::from_items(child_hashes.clone());
                let subgraph_hash = subgraph_hash
                    .sort()
                    .push(write_lock.hash.clone())
                    .finalize();

                write_lock.subgraph_hash = subgraph_hash.clone();
                write_lock.children.extend(children);
            }

            graph_node
        }

        let graph_root = recurse(
            Arc::new(dummy_node),
            Vec::new(),
            &mut schema_nodes,
            &mut schema_contexts,
        );

        Ok((schema_contexts, graph_root))
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
        let path = Path::from_str(&name);

        //




        let properties = if let Some(props) = serde_value["properties"].as_object() {
            props
                .iter()
                .map(|(key, val)| {
                    match SchemaNode::from_serde_value(
                        &val,
                        &key,
                        &lineage,
                        &path,
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

    pub fn get_schema_node_by_json_path(&self, json_path: &str) -> Option<&SchemaNode> {
        log::trace!("In get_schema_node_by_json_path");

        let path = json_path.strip_prefix("$.").unwrap_or(json_path);
        let mut segments = path.split('.');

        if segments.next() != Some(&self.name) {
            log::warn!("Could not obtain schema node using json path: {}", json_path);
            return None;
        }

        let mut current_schema_nodes: Vec<&SchemaNode> = self.properties.values().collect();

        while let Some(segment) = segments.next() {
            let mut next_schema_nodes = Vec::new();

            for node in &current_schema_nodes {
                match segment {
                    "properties" => {
                        if let Some(property_name) = segments.next() {
                            if let Some(matching_node) = node.properties.get(property_name) {
                                next_schema_nodes.push(matching_node);
                            }
                        }
                    }
                    "items" => {
                        if let Some(item_name) = segments.next() {
                            if let Some(items) = &node.items {
                                for item in items {
                                    if item.name == item_name {
                                        next_schema_nodes.push(item);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        log::warn!("Unexpected segment: {}", segment);
                        return None;
                    }
                }
            }

            if next_schema_nodes.is_empty() {
                return None;
            }

            current_schema_nodes = next_schema_nodes;
        }

        current_schema_nodes.into_iter().next()
    }
}
