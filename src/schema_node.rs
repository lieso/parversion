use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    #[serde(skip_serializing)]
    pub id: ID,
    #[serde(skip_serializing)]
    pub name: String,
    pub description: String,
    pub data_type: String,
    pub properties: HashMap<String, SchemaNode>,
}
