use serde::{
    Serialize,
    Deserialize,
    Deserializer,
    de::Error as DeError,
    Serializer,
    ser::Error as SerError
};
use serde::ser::SerializeStruct;

use crate::graph_node::{Graph, GraphNode};
use crate::basis_node::{BasisNode};
use crate::macros::*;

#[derive(Clone, Debug)]
pub struct BasisGraph {
    pub root: Graph<BasisNode>,
    pub subgraph_hashes: Vec<String>,
}

impl<'de> Deserialize<'de> for BasisGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BasisGraphHelper {
            root: String,
            subgraph_hashes: Vec<String>,
        }

        let helper = BasisGraphHelper::deserialize(deserializer)?;

        let root: Graph<BasisNode> = GraphNode::deserialize(&helper.root).map_err(DeError::custom)?;

        Ok(BasisGraph {
            root,
            subgraph_hashes: helper.subgraph_hashes,
        })
    }
}

impl Serialize for BasisGraph {
     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
     where
         S: Serializer,
     {
         let basis_root = read_lock!(self.root);
         let root_str = GraphNode::serialize(&basis_root).map_err(SerError::custom)?;

         let mut state = serializer.serialize_struct("BasisGraph", 2)?;
         state.serialize_field("root", &root_str)?;
         state.serialize_field("subgraph_hashes", &self.subgraph_hashes)?;
         state.end()
     }
 }
