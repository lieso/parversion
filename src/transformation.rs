use quick_js::Context as QuickContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::id::ID;
use crate::json_node::{Json, JsonNode};
use crate::path::Path;
use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::basis_network::BasisNetwork;
use crate::traversal::Traversal;
use crate::network_relationship::NetworkRelationshipType;
use crate::graph_node::Graph;

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
    pub timestamp: Timestamp,
    pub lineage: Lineage,
    pub subgraph_hash: Option<Hash>,
    pub key: String,
    pub description: String,
    pub source: Option<Path>,
    pub target: Option<Path>,
}

impl SchemaTransformation {
    pub fn transform(&self, schema_node: &SchemaNode) -> SchemaNode {
        let mut transformed = schema_node.clone();
        transformed.name = self.key.clone();
        transformed.description = self.description.clone();

        transformed
    }
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
                        let escaped_value = value.replace("\"", "\\\"");

                        if key == "text" {
                            format!("'{}': '<omitted>'", key)
                        } else {
                            format!("'{}': \"{}\"", key, escaped_value)
                        }
                    })
                    .collect();
                format!("let fields = {{ {} }};", fields_js.join(", "))
            }
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }

    fn suffix(&self) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                format!("JSON.stringify({{ hasherItems }})")
            }
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }

    pub fn transform(&self, fields: HashMap<String, String>) -> Hash {
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
            }
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

        let attributes_json =
            serde_json::to_string(&attributes).expect("Could not serialize attributes");
        let attributes_code = format!("let attributes = {};", attributes_json);

        format!("{}\n{}", element_code, attributes_code)
    }

    fn suffix(&self) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                format!("JSON.stringify({{ element, attributes }})")
            }
            _ => panic!("unexpected runtime: {:?}", self.runtime),
        }
    }

    pub fn transform(
        &self,
        element: String,
        attributes: HashMap<String, String>,
    ) -> (Option<String>, HashMap<String, String>) {
        let prefix = self.prefix(element, attributes);
        let suffix = self.suffix();

        let code = format!("{}\n{}\n{}", prefix, self.infix, suffix);

        match self.runtime {
            Runtime::QuickJS => {
                let quick_context = QuickContext::new().unwrap();

                let result = quick_context.eval_as::<String>(&code).unwrap();

                let parsed: Value = serde_json::from_str(&result).unwrap();

                let transformed_element = parsed
                    .get("element")
                    .and_then(|e| e.as_str().map(String::from));

                let transformed_attributes = parsed
                    .get("attributes")
                    .and_then(|attr| attr.as_object())
                    .map(|attr_obj| {
                        attr_obj
                            .iter()
                            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                            .collect::<HashMap<String, String>>()
                    })
                    .unwrap_or_default();

                (transformed_element, transformed_attributes)
            }
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldMetadata {
    pub data_type: String,
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
        if let Some(value) = data_node.fields.get(&self.field) {
            let json = Json {
                key: self.image.clone(),
                value: value.to_string(),
            };

            let json_node = JsonNode {
                id: ID::new(),
                hash: data_node.hash.clone(),
                lineage: data_node.lineage.clone(),
                description: self.description.clone(),
                json,
            };

            Ok(json_node)
        } else {
            Err(Errors::FieldTransformationFieldNotFound)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkMetadata {
    pub fields: Vec<String>,
    pub cardinality: String,
    pub field_types: Vec<String>,
    pub context: String,
    pub structure: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkTransformation {
    pub id: ID,
    pub description: String,
    pub subgraph_hash: String,
    pub image: String,
    pub meta: NetworkMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalizationTransformation {
    pub id: ID,
    pub canonical_networks: Vec<String>,
}

impl CanonicalizationTransformation {
    pub fn transform(
        &self,
        networks: Vec<Arc<BasisNetwork>>
    ) -> Result<Vec<Arc<BasisNetwork>>, Errors> {
        Ok(
            networks
                .into_iter()
                .filter(|network| {
                    self.canonical_networks.contains(&network.subgraph_hash.to_string().unwrap())
                })
                .collect()
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelationshipTransformation {
    pub id: ID,
    pub from: ID,
    pub to: ID,
    pub relationship_type: NetworkRelationshipType,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedRelationshipTransformation {
    pub id: ID,
    pub from: Arc<BasisNetwork>,
    pub to: Arc<BasisNetwork>,
    pub relationship_type: NetworkRelationshipType,
    pub description: String,
}

impl RelationshipTransformation {
    pub fn transform(
        &self,
        networks: &[Arc<BasisNetwork>]
    ) -> Result<ResolvedRelationshipTransformation, Errors> {
        let from = networks.iter()
            .find(|n| n.id == self.from)
            .ok_or(Errors::UnexpectedError)?
            .clone();
        let to = networks.iter()
            .find(|n| n.id == self.to)
            .ok_or(Errors::UnexpectedError)?
            .clone();

        Ok(ResolvedRelationshipTransformation {
            id: self.id.clone(),
            from,
            to,
            relationship_type: self.relationship_type.clone(),
            description: self.description.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TraversalTransformation {
    pub id: ID,
    pub relationship_id: ID,
    pub traversal: Traversal,
    pub name: String,
    pub description: String,
}

impl TraversalTransformation {
    pub fn transform(
        &self,
        meta_context: Arc<RwLock<MetaContext>>,
        start: Graph,
    ) -> Result<Option<Graph>, Errors> {
        use crate::graph_node::GraphNode;

        GraphNode::traverse_using_xpath(
            meta_context,
            start,
            &self.traversal.candidate,
        )
    }
}
