use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::FieldTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: ID,
    pub hash: Hash,
    pub acyclic_lineage: Lineage,
    pub lineage: Option<Lineage>,
    pub indexed_lineage: Option<Lineage>,
    pub description: String,
    pub transformations: Vec<FieldTransformation>,
}
