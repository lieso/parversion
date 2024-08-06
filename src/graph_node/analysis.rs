use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{GraphNode, Graph, GraphNodeData, bft, get_lineage};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};

pub async fn analyze_structure(
    graph: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_structure");

    let lineage = get_lineage(Arc::clone(&graph));
    log::debug!("lineage: {:?}", lineage);

    unimplemented!()
}
