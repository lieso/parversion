use serde::{Deserialize, Serialize};

use crate::node_relationship::{NodeRelationship, NodeRelationshipType};
use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraversalMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TraversalKind {
    XPath {
        xpath_ltr: String,
        xpath_rtl: String,
    },
    Xslt {
        xslt_ltr: String,
        xslt_rtl: String,
    },
}

// TODO: eliminate ambiguity
// xslt_ltr implies parent-to-child
// xslt_rtl implies child-to-parent

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Traversal {
    pub id: ID,
    pub left_basis_lineage: Option<Lineage>,
    pub right_basis_lineage: Option<Lineage>,
    pub kind: TraversalKind,
    pub metadata: TraversalMetadata,
}

impl Traversal {
    pub fn from_node_relationship(relationship: &NodeRelationship) -> Result<Self, Errors> {
        match relationship.relationship_type.clone() {
            NodeRelationshipType::Combine { traversal } => Ok(traversal),
            NodeRelationshipType::MixedContent { traversal } => Ok(traversal),
            NodeRelationshipType::Equal { traversal } => Ok(traversal),
            NodeRelationshipType::NoRelationship => {
                Err(Errors::UnexpectedError(
                    "Attempting to create a Traversal from a NoRelationship relationship"
                        .to_string(),
                ))
            }
        }
    }
}
