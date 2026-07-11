use serde::{Deserialize, Serialize};

use crate::prelude::*;

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
