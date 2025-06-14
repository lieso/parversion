use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    pub id: ID,
    pub name: String,
    pub hash: Hash,
    pub lineage: Lineage,
    pub aliases: Vec<String>,
    pub description: String,
    pub data_type: String,
    pub properties: HashMap<String, SchemaNode>,
}

impl SchemaNode {
    pub fn new(
        name: &str,
        description: &str,
        parent_lineage: &Lineage,
        data_type: &str,
    ) -> Self {
        let hash: Hash = Hash::from_str(&name);
        let lineage = parent_lineage.with_hash(hash.clone());

        SchemaNode {
            id: ID::new(),
            hash,
            lineage,
            name: name.to_string(),
            aliases: Vec::new(),
            description: description.to_string(),
            data_type: data_type.to_string(),
            properties: HashMap::new(),
        }
    }
}
