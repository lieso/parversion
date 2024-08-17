use std::sync::{Arc};
use serde::{Serialize, Deserialize};

use crate::graph_node::{Graph};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::error::{Errors};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Graph<XmlNode>,
    pub basis_graph: Option<Graph<BasisNode>>,
}

impl Traversal {
    pub fn from_tree(output_tree: Graph<XmlNode>) -> Self {
        Traversal {
            output_tree: output_tree,
            basis_graph: None,
        }
    }

    pub fn with_basis(mut self, graph: Graph<BasisNode>) -> Self {
        self.basis_graph = Some(Arc::clone(&graph));

        self
    }

    pub fn harvest(self) -> Result<String, Errors> {
        unimplemented!()
    }

}
