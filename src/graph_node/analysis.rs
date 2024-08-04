use tokio::sync::{OwnedSemaphorePermit};

use super::{GraphNode, Graph, GraphNodeData, bft};

pub async fn analyze_structure<T: GraphNodeData>(graph: Graph<T>, _permit: OwnedSemaphorePermit) {
    log::trace!("In analyze_structure");
    unimplemented!()
}
