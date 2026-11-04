use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_node::{DataNode, DataNodeFields, FieldValue};
use crate::prelude::*;
use crate::scheme::{eval, apply, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transformation {
    pub id: ID,
    pub description: String,
    pub field: String,
    pub procedure: Option<String>,
    pub image: String,
}

impl Transformation {
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<DataNode, Errors> {
        let mut fields = DataNodeFields::new();

        let compiled = match &self.procedure {
            Some(source) => Some(eval(source)?),
            None => None,
        };

        for value in data_node.fields.get(&self.field) {
            let out = match &compiled {
                Some(procedure) => {
                    apply(procedure, vec![Value::from_field_value(value.clone())])?.to_field_value()?
                }
                None => value.clone(),
            };
            fields.insert(self.image.clone(), out);
        }

        let transformed = DataNode {
            id: ID::new(),
            description: self.description.clone(),
            fields,
        };

        Ok(transformed)
    }
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
        let mut fields = DataNodeFields::new();

        for value in data_node.fields.get(&self.field) {
            fields.insert(self.image.clone(), FieldValue::Text(value.to_string()));
        }

        let transformed = DataNode {
            id: ID::new(),
            description: self.description.clone(),
            fields,
        };

        Ok(transformed)
    }
}
