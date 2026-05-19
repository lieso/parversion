use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::BasisFieldTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisField {
    pub id: ID,
    pub lineage: Lineage,
    pub transformation: BasisFieldTransformation
}
