use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::transformation::NetworkTransformation;
use crate::graph_node::{Graph, GraphNode};
use crate::normal_context::NormalContext;
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetworkMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: ID,
    pub basis_lineages: Hash,
    pub transformation: NetworkTransformation,
    pub metadata: BasisNetworkMetadata,
}

impl BasisNetwork {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        context: Arc<Context>,
        parent: Graph
    ) -> Result<NormalContext, Errors> {
        let start_node = context.graph_node.clone();

        fn recurse(
            normalization_context: Arc<RwLock<NormalizationContext>>,
            current_network: &BasisNetwork,
            current: Graph,
        ) -> Result<Vec<DataNode>, Errors> {
            let contexts = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };
            let context = contexts.get(&read_lock!(current).id).unwrap();

            let context_to_network = {
                let lock = read_lock!(normalization_context);
                lock.context_basis_networks.clone().unwrap()
            };

            if let Some(basis_network) = context_to_network.get(&context.id) {
                if basis_network.id != current_network.id {
                    log::info!("Detected network boundary");
                    return Ok(Vec::new());
                }
            }

            let children = read_lock!(current).children.clone();

            let data_nodes: Vec<DataNode> = children
                .iter()
                .map(|child| recurse(
                    Arc::clone(&normalization_context),
                    current_network,
                    Arc::clone(&child),
                ))
                .collect::<Result<Vec<Vec<DataNode>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            let context_to_group = {
                let lock = read_lock!(normalization_context);
                lock.context_to_group.clone().unwrap()
            };

            if let Some(basis_group) = context_to_group.get(&context.id).cloned() {
                if let Some(data_node) = basis_group.apply(
                    Arc::clone(&normalization_context),
                    Arc::clone(&context),
                )? {
                    return Ok(
                        data_nodes
                            .into_iter()
                            .chain(std::iter::once(data_node))
                            .collect()
                    );
                }
            }

            Ok(data_nodes)
        }

        let data_nodes = recurse(
            Arc::clone(&normalization_context),
            &self,
            Arc::clone(&start_node),
        )?;

        let combined_data_node = Arc::new(DataNode::from_data_nodes(data_nodes));

        let graph_node = Arc::new(RwLock::new(
            GraphNode::from_data_node(Arc::clone(&combined_data_node), vec![Arc::clone(&parent)])
        ));

        write_lock!(parent).children.push(Arc::clone(&graph_node));
       
        Ok(NormalContext {
            id: ID::new(),
            network_name: Some(self.transformation.image.clone()),
            network_description: Some(self.transformation.description.clone()),
            data_node: combined_data_node,
            graph_node,
        })
    }
}
