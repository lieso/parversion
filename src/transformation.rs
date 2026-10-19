use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_node::{DataNode, DataNodeFields};
use crate::prelude::*;

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
        let mut fields = DataNodeFields::new();

        for value in data_node.fields.get(&self.field) {
            fields.insert(self.image.clone(), value.to_string());
        }

        let transformed = DataNode {
            id: ID::new(),
            description: self.description.clone(),
            fields,
        };

        Ok(transformed)
    }
}
