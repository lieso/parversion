use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::basis_network::BasisNetwork;
use crate::traversal::Traversal;
use crate::graph_node::{Graph};
use crate::normal_meta_context::NormalMetaContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphNode {
    pub id: ID,
    pub parent: Option<Arc<RwLock<BasisGraphNode>>>,
    pub basis_network: Arc<BasisNetwork>,
    pub traversal: Option<Traversal>,
    pub children: Vec<Arc<RwLock<BasisGraphNode>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub name: Option<String>,
    pub description: Option<String>,
    pub lineage: Lineage,
    pub graph_roots: Vec<Arc<RwLock<BasisGraphNode>>>,
    pub metadata: BasisGraphMetadata,
}

impl BasisGraph {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        parent: Graph,
    ) -> Result<NormalMetaContext, Errors> {
        log::trace!("In apply");

        for graph_root in self.graph_roots.clone() {

            let basis_network = &read_lock!(graph_root).basis_network;



            let normal_meta_context = basis_network.apply(
                Arc::clone(&normalization_context),
                Arc::clone(&parent)
            )?;




            for child in &read_lock!(graph_root).children {

                if let Some(traversal) = &read_lock!(child).traversal {



                }
                

            }



        }

        unimplemented!()
    }
}
