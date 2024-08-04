use tokio::sync::{OwnedSemaphorePermit};

use super::{GraphNode, Graph, GraphNodeData, bft};
use crate::xml_node::{XmlNode};

pub async fn analyze_structure<T: GraphNodeData>(graph: Graph<T>, output_tree: Graph<XmlNode>, _permit: OwnedSemaphorePermit) {
    log::trace!("In analyze_structure");
    unimplemented!()
}
