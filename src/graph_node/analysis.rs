use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{GraphNode, Graph, GraphNodeData, bft, find_homologous_nodes};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};

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

    unimplemented!()
}
