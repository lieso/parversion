use serde::{Serialize, Deserialize};

use crate::transformation::{Transformation};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: String,
    pub hash: String,
    pub lineage: String,
    pub description: String,
    pub transformations: Vec<Transformation>,
}
