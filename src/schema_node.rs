use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    id: ID,
    name: String,
    description: String,
    json_path: Vec<String>,
    data_type: String,
}
