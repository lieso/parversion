use serde::{Deserialize, Serialize};

use crate::node_relationship::{NodeRelationship, NodeRelationshipType};
use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TraversalKind {
    XPath { xpath_ltr: String, xpath_rtl: String },
    Xslt { xslt_ltr: String, xslt_rtl: String },
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
            id: ID::new(),
            left_basis_lineage: Some(relationship.left_basis_lineage.clone()),
            right_basis_lineage: Some(relationship.right_basis_lineage.clone()),
            kind: TraversalKind::XPath { xpath_ltr, xpath_rtl }
        })
    }
}
