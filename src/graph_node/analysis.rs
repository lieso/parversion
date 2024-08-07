use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{GraphNode, Graph, GraphNodeData, bft, find_homologous_nodes};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::macros::*;
use crate::config::{CONFIG, Config};

pub async fn analyze_structure(
    target_node: Graph<BasisNode>,
    basis_graph: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_structure");

    let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
        Arc::clone(&target_node),
        Arc::clone(&basis_graph),
        Arc::clone(&output_tree),
    );

    if homologous_nodes.is_empty() {
        panic!("There cannot be zero homologous nodes for any basis node with respect to output tree.");
    }

    for node in homologous_nodes.iter() {
        log::debug!("homologous node: {}", read_lock!(node).data.describe());
    }

    let target_node_examples_max_count = read_lock!(CONFIG).llm.target_node_examples_max_count.clone();
    log::info!("Using {} examples of target node for analysis", target_node_examples_max_count);

    let homologous_nodes = homologous_nodes[..target_node_examples_max_count].to_vec();


    unimplemented!()
}
