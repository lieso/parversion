use serde::{Serialize, Deserialize};

use crate::transformation::{Transformation};
use crate::hash::{Hash};
use crate::id::{ID};
use crate::lineage::{Lineage};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub description: String,
    pub transformations: Vec<Transformation>,
}
