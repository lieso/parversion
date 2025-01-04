use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::transformation::HashTransformation;

pub type DataNodeFields = HashMap<String, String>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNode {
    pub id: ID,
    pub context_id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub fields: DataNodeFields,
    pub description: String,
}

impl DataNode {
    pub fn new(
        hash_transformation: &HashTransformation,
        context_id: ID,
        fields: DataNodeFields,
        description: String,
        parent_lineage: &Lineage,
    ) -> Self {
        let hash: Hash = hash_transformation.transform(fields.clone());
        let lineage = parent_lineage.with_hash(hash.clone());

        DataNode {
            id: ID::new(),
            hash,
            context_id,
            fields,
            lineage,
            description,
        }
    }

    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}
