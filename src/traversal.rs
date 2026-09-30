use serde::{Deserialize, Serialize};

use crate::node_relationship::{NodeRelationship, NodeRelationshipType};
use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Traversal {
    pub left_basis_lineage: Lineage,
    pub right_basis_lineage: Lineage,
    pub xpath_ltr: String,
    pub xpath_rtl: String,
}

impl Traversal {
    pub fn from_node_relationship(relationship: &NodeRelationship) -> Result<Self, Errors> {
        let (xpath_ltr, xpath_rtl) = {
            match relationship.relationship_type.clone() {
                NodeRelationshipType::Combine {
                    xpath_ltr,
                    xpath_rtl,
                } => (xpath_ltr, xpath_rtl),
                NodeRelationshipType::MixedContent {
                    xpath_ltr,
                    xpath_rtl,
                } => (xpath_ltr, xpath_rtl),
                NodeRelationshipType::Equal {
                    xpath_ltr,
                    xpath_rtl,
                } => (xpath_ltr, xpath_rtl),
                NodeRelationshipType::NoRelationship => {
                    return Err(Errors::UnexpectedError(
                        "Attempting to create a Traversal from a NoRelationship relationship"
                            .to_string(),
                    ));
                }
            }
        };

        Ok(Traversal {
            left_basis_lineage: relationship.left_basis_lineage.clone(),
            right_basis_lineage: relationship.right_basis_lineage.clone(),
            xpath_ltr,
            xpath_rtl,
        })
    }
}
