use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::FieldTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNodeMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: ID,
    pub lineage: BasisLineage,
    pub transformations: Vec<FieldTransformation>,
    pub metadata: BasisNodeMetadata,
}
