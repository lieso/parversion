use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::traversal::Traversal;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NodeRelationshipType {
    Combine { traversal: Traversal },
    Equal { traversal: Traversal },
    MixedContent { traversal: Traversal },
    NoRelationship,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeRelationship {
    pub id: ID,
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub relationship_type: NodeRelationshipType,
}
