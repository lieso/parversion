use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BasisGroupMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGroup {
    pub id: ID,
    pub hash: Hash,
    pub acyclic_lineage: Lineage,
    pub lineage: Option<Lineage>,
    pub indexed_lineage: Option<Lineage>,
    #[serde(default)]
    pub metadata: BasisGroupMetadata,
}

impl BasisGroup {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        context: Arc<Context>
    ) -> Result<Option<DataNode>, Errors> {
        let basis_lineage = self.get_basis_lineage();

        let basis_node = {
            let lock = read_lock!(normalization_context);
            lock.get_basis_node_by_lineage(&basis_lineage)
                .expect("Could not get basis node by lineage")
                .unwrap()
        };

        Ok(basis_node.apply(
            Arc::clone(&context)
        )?)
    }

    pub fn get_basis_lineage(&self) -> BasisLineage {
        let mut hashes: Vec<Hash> = vec![self.acyclic_lineage.identity_hash.clone()];

        if let Some(lineage) = &self.lineage {
            hashes.push(lineage.identity_hash.clone());
        }

        if let Some(indexed_lineage) = &self.indexed_lineage {
            hashes.push(indexed_lineage.identity_hash.clone());
        }

        Lineage::from_hashes(hashes)
    }
}
