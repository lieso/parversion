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
    pub procedure: Option<String>,
    pub image: String,
}

impl Transformation {
    pub fn transform(&self, data_node: Arc<DataNode>) -> Result<DataNode, Errors> {
        let mut fields = DataNodeFields::new();

        let compiled = match &self.procedure {
            Some(source) => {
                let program = eval(source).map_err(|e| {
                    log::error!(
                        "Failed to compile Scheme program during transformation: {:?}",
                        self.procedure
                    );
                    e
                })?;

                Some(program)
            },
            None => None,
        };

        for value in data_node.fields.get(&self.field) {
            let out = match &compiled {
                Some(procedure) => {
                    let arguments = vec![Value::from_field_value(value.clone())];
                    let transformed_value = apply(procedure, arguments).map_err(|e| {
                        log::error!(
                            "Failed to apply procedure when transforming value: {:?}",
                            self.procedure
                        );
                        e
                    })?;

                    transformed_value.to_field_value()?
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
