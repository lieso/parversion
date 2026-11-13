use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_node::{DataNode, DataNodeFields, FieldValue};
use crate::prelude::*;
use crate::scheme::{apply, eval, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transformation {
    pub id: ID,
    pub description: String,
    pub field: String,
    pub image: String,
    pub forward: String,
    pub inverse: String,
}

impl Transformation {
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<DataNode, Errors> {
        let mut fields = DataNodeFields::new();

        let compiled = eval(&self.forward).map_err(|e| {
            log::error!(
                "Failed to compile Scheme program during transformation: {:?}",
                self.forward
            );
            e
        })?;

        for value in data_node.fields.get(&self.field) {
            let arguments = vec![Value::from_field_value(value.clone())];
            let transformed_value = apply(&compiled, arguments).map_err(|e| {
                log::error!(
                    "Failed to apply procedure when transforming value: {:?}",
                    self.forward
                );
                e
            })?;

            let out = transformed_value.to_field_value()?;

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
