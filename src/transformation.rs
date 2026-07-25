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
use crate::graph_node::Graph;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Runtime {
    AWK,
    NodeJS,
    Python,
    QuickJS,
}




#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkTranslationTransformation {
    pub id: ID,
    pub image: String,
    pub cardinality: String,
}







#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldTranslationTransformation {
    pub id: ID,
    pub field: String,
    pub image: String,
    pub code: Option<String>,
}

impl FieldTranslationTransformation {
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<DataNode, Errors> {
        let fields = {
            if let Some(value) = data_node.fields.get(&self.field) {
                let mut fields = HashMap::new();
                fields.insert(self.image.clone(), value.to_string());

                fields
            } else {
                HashMap::new()
            }
        };

        let transformed = DataNode {
            id: ID::new(),
            hash: data_node.hash.clone(),
            lineage: data_node.lineage.clone(),
            description: data_node.description.clone(),
            fields,
        };

        Ok(transformed)
    }
}













#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisFieldTransformation {
    pub id: ID,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldMetadata {
    pub data_type: String,
    pub format: Option<String>,
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
pub struct NetworkTransformation {
    pub id: ID,
    pub description: String,
    pub image: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelationshipTransformation {
    pub id: ID,
}
