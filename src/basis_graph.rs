use std::sync::{Arc};
use std::io::{Write};
use serde::{
    Serialize,
    Deserialize,
    Deserializer,
    de::Error as DeError,
    Serializer,
    ser::Error as SerError
};
use serde::ser::SerializeStruct;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use uuid::Uuid;

use parversion::environment;
use parversion::graph_node::{
    Graph,
    GraphNode,
    to_xml_string,
    deep_copy,
    graph_hash
};
use parversion::basis_node::{BasisNode};
use parversion::macros::*;
use parversion::xml_node::{XmlNode};
use parversion::llm::{analyze_compressed_website};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Subgraph {
    pub id: String,
    pub hash: String,
    pub title: String,
    pub description: String,
    pub has_recursive: bool,
    pub analyzed: bool,
}

#[derive(Clone, Debug)]
pub struct BasisGraph {
    pub root: Graph<BasisNode>,
    pub subgraphs: HashMap<String, Subgraph>,
}

impl BasisGraph {
    pub fn contains_subgraph(&self, graph: Graph<XmlNode>) -> bool {
        let hash = graph_hash(Arc::clone(&graph));

        self.subgraphs.contains_key(&hash)
    }
}

pub fn build_basis_graph(input_graph: Graph<XmlNode>) -> BasisGraph {
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
        subgraphs: HashMap::new(),
    }
}

pub async fn analyze_graph(graph: &mut BasisGraph, input_graph: Graph<XmlNode>) {
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

    let title = get_graph_title(Arc::clone(&input_graph)).unwrap();
    log::debug!("title: {}", title);

    let summary = analyze_compressed_website(pruned_input).await;
    log::debug!("summary: {:?}", summary);

    let subgraph = Subgraph {
        id: Uuid::new_v4().to_string(),
        hash: subgraph_hash.clone(),
        description: summary.core_purpose,
        has_recursive: summary.has_recursive,
        title,
        analyzed: false,
    };

    graph.subgraphs.entry(subgraph_hash.clone()).or_insert(subgraph);
}

impl<'de> Deserialize<'de> for BasisGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BasisGraphHelper {
            root: String,
            subgraphs: HashMap<String, Subgraph>,
        }

        let helper = BasisGraphHelper::deserialize(deserializer)?;

        let root: Graph<BasisNode> = GraphNode::deserialize(&helper.root).map_err(DeError::custom)?;

        Ok(BasisGraph {
            root,
            subgraphs: helper.subgraphs,
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
        state.serialize_field("subgraphs", &self.subgraphs)?;
        state.end()
    }
}

fn get_graph_title(root: Graph<XmlNode>) -> Option<String> {
    log::trace!("In get_graph_title");

    if let Some(head) = read_lock!(root).children.iter().find(|child| {
        read_lock!(child).data.get_element_tag_name() == "head"
    }) {
        if let Some(title) = read_lock!(head).children.iter().find(|child| {
            read_lock!(child).data.get_element_tag_name() == "title"
        }) {
            if let Some(text) = read_lock!(title).children.first() {
                let title_text = read_lock!(text).data.to_string();

                return Some(title_text);
            }
        }
    }

    None
}
