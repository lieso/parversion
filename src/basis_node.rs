use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::transformation::{Transformation};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub description: String,
    pub transformations: Vec<Transformation>,
}
