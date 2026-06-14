use quick_js::Context as QuickContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::id::ID;
use crate::prelude::*;
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
pub struct FieldTranslationTransformation {
    pub id: ID,
    pub field: String,
    pub image: String,
    pub code: String,
}













#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisFieldTransformation {
    pub id: ID,
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
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<DataNode, Errors> {
        if let Some(value) = data_node.fields.get(&self.field) {
            let mut fields = HashMap::new();
            fields.insert(self.image.clone(), value.to_string());

            let transformed = DataNode {
                id: ID::new(),
                hash: data_node.hash.clone(),
                lineage: data_node.lineage.clone(),
                description: self.description.clone(),
                fields,
            };

            Ok(transformed)
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
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Option<Graph>, Errors> {
        use crate::graph_node::GraphNode;

        GraphNode::traverse_using_xpath(
            normalization_context,
            start,
            &self.traversal.candidate,
        )
    }
}
