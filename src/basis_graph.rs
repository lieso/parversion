use serde::{
    Serialize,
    Deserialize,
    Deserializer,
    de::Error as DeError,
    Serializer,
    ser::Error as SerError
};
use serde::ser::SerializeStruct;

use crate::graph_node::{
    Graph,
    GraphNode,
    to_xml_string,
    deep_copy,
    graph_hash
};
use crate::basis_node::{BasisNode};
use crate::macros::*;

#[derive(Clone, Debug)]
pub struct BasisGraph {
    pub root: Graph<BasisNode>,
    pub subgraph_hashes: Vec<String>,
    pub description: Option<String>,
}

pub fn build_basis_graph(input_graph: Graph<XmlNode>) {
    log::trace!("In build_basis_graph");

    let copy: Graph<BasisNode> = deep_copy(
        Arc::clone(&input_graph),
        vec![GraphNode::from_void()],
        &mut HashSet::new(),
        &mut HashMap::new()
    );

    let new_root: Graph<BasisNode> = GraphNode::from_void();

    {
        write_lock!(new_root).children.push(Arc::clone(&copy));
    }

    BasisGraph {
        root: new_root,
        subgraphs: Vec::new(),
    }
}

pub async fn analyze_graph(graph: BasisGraph, input_graph: Graph<XmlNode>) {
    log::trace!("In analyze_graph");

    read_lock!(input_graph).debug_statistics("pruned_input_graph");
    read_lock!(input_graph).debug_visualize("pruned_input_graph");

    let pruned_input: String = to_xml_string(Arc::clone(&input_graph));

    if environment::is_local() {
        let mut file = File::create("./debug/pruned_input.xml").expect("Could not create file");
        file.write_all(pruned_input.as_bytes()).expect("Could not write to file");
    }




    let subgraph_hash = graph_hash(Arc::clone(&input_graph));
    log::debug!("subgraph_hash: {}", subgraph_hash);





    unimplemented!()
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
