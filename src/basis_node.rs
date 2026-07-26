use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::data_node::DataNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNodeMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: ID,
    pub lineage: BasisLineage,
    pub transformations: Vec<FieldTransformation>,
    pub metadata: BasisNodeMetadata,
}

impl BasisNode {
    pub fn apply(
        &self,
        context: Arc<Context>
    ) -> Result<Option<DataNode>, Errors> {
        let data_node = &context.data_node;

        let transformed: Vec<DataNode> = self
            .transformations
            .clone()
            .into_iter()
            .map(|transformation| {
                transformation
                    .transform(Arc::clone(&data_node))
                    .expect("Could not transform data node")
            })
            .collect();

        if transformed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DataNode::from_data_nodes(transformed)))
        }
    }
}
