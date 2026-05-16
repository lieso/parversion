use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGroup {
    pub id: ID,
    pub acyclic_lineage: Lineage,
    pub lineage: Option<Lineage>,
    pub indexed_lineage: Option<Lineage>,
}
