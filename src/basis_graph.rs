use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::basis_network::BasisNetwork;
use crate::graph_node::{Graph, GraphNode};
use crate::normal_meta_context::NormalMetaContext;
use crate::traversal::{Traversal, TraversalKind};
use crate::xslt::Xslt;

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


            let temp_parent = Arc::new(RwLock::new(GraphNode {
                id: ID::new(),
                parents: Vec::new(),
                description: String::from("placeholder description"),
                hash: Hash::new(),
                subgraph_hash: Hash::new(),
                lineage: Lineage::new(),
                children: Vec::new(),
            }));


            let normal_meta_context = basis_network.apply(
                Arc::clone(&normalization_context),
                Arc::clone(&temp_parent)
            )?;




            for child in &read_lock!(graph_root).children {


                if let Some(traversal) = &read_lock!(child).traversal {



                    let instance_roots = read_lock!(normal_meta_context.graph_root).children.clone();





                    for instance_root in instance_roots {
                        let normal_context = {
                            normal_meta_context
                                .contexts_lookup
                                .get(&read_lock!(instance_root).id)
                                .unwrap()
                                .clone()
                        };

                        let contexts = normal_context.contexts.clone();

                        let (xslt_ltr, xslt_rtl) = {
                            match &traversal.kind {
                                TraversalKind::XPath { .. } => {
                                    unimplemented!()
                                }
                                TraversalKind::Xslt { xslt_ltr, xslt_rtl } => {
                                    (xslt_ltr.clone(), xslt_rtl.clone())
                                }
                            }
                        };

                        let xslt: Xslt = Xslt::new(&xslt_rtl)?;


                        for context in contexts {

                            let target_values = xslt.traverse(
                                Arc::clone(&normalization_context),
                                Arc::clone(&context.graph_node),
                            )?;

                            let target_graph_nodes: Vec<Graph> = target_values.into_iter().map(|v| v.graph).collect();

                            if target_graph_nodes.is_empty() {
                                log::warn!("Could not find target graph nodes");
                            } else {
                                log::info!("Found a graph node using xslt");
                            }

                        }



                    }







                }
                

            }



        }

        unimplemented!()
    }
}
