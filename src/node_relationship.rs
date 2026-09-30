use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeRelationshipType {
    Combine {
        xpath_ltr: String,
        xpath_rtl: String,
    },
    Equal {
        xpath_ltr: String,
        xpath_rtl: String,
    },
    MixedContent {
        xpath_ltr: String,
        xpath_rtl: String,
    },
    NoRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeRelationship {
    pub id: ID,
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub relationship_type: NodeRelationshipType,
}
