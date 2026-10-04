use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationshipType {
    ParentChild {
        xslt_parent_to_child: String,
        xslt_child_to_parent: String,
    },
    NoRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationship {
    pub id: ID,
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub relationship_type: NetworkRelationshipType,
}
