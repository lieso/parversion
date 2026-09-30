use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Traversal {
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub xpath_ltr: String,
    pub xpath_rtl: String,
}

