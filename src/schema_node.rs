use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub data_type: String,
    pub children: HashMap<String, SchemaNode>,
}
