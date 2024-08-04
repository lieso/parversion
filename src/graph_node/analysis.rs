use tokio::sync::{OwnedSemaphorePermit};

use super::{GraphNode, Graph, GraphNodeData, bft};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};

pub async fn analyze_structure(
    graph: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_structure");

    unimplemented!()
}
