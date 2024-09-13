use tokio::runtime::Runtime;
use serde_json::{
    json,
    to_string_pretty,
    Result as SerdeResult,
    Value as JsonValue,
    from_str
};
use std::fs::{File};
use std::process;
use std::io::{Read};
use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};

mod error;
mod llm;
mod node_data;
mod node_data_structure;
mod utility;
mod xml_node;
mod config;
mod constants;
mod basis_node;
mod graph_node;
mod macros;
mod traversal;

pub use graph_node::GraphNodeData;
pub use graph_node::GraphNode;
pub use graph_node::Graph;
pub use basis_node::BasisNode;

use graph_node::{
    absorb,
    cyclize,
    prune,
    interpret,
};
use xml_node::{XmlNode};
use error::{Errors};
use traversal::{Traversal, Harvest};

#[derive(Debug)]
pub enum HarvestFormats {
    JSON,
    //XML,
    //CSV,
    //HTML
}

pub struct NormalizeResult {
    pub basis_graph: Graph<BasisNode>,
    pub harvest: Harvest,
}

pub fn normalize_text(
    text: String,
    input_basis_graph: Option<Graph<BasisNode>>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_text");

    if text.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    return Runtime::new().unwrap().block_on(async {
        if utility::is_valid_xml(&text) {
            log::info!("Document is valid XML");

            let result = normalize_xml(&text, input_basis_graph).await?;

            return Ok(result);
        }

        if let Some(xml) = utility::string_to_xml(&text) {
            log::info!("Managed to convert string to XML");

            let result = normalize_xml(&xml, input_basis_graph).await?;

            return Ok(result);
        }

        Err(Errors::UnexpectedDocumentType)
    });
}

pub fn normalize_file(
    file_name: &str,
    input_basis_graph: Option<Graph<BasisNode>>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file_name: {}", file_name);

    let mut document = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut document).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    normalize_text(document, input_basis_graph)
}

pub async fn normalize_xml(
    xml: &str,
    input_basis_graph: Option<Graph<BasisNode>>
) -> Result<NormalizeResult, Errors> {
    log::trace!("In normalize_xml");

    let xml = utility::preprocess_xml(xml);
    log::info!("Done preprocessing XML");

    let input_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());
    let output_tree: Graph<XmlNode> = graph_node::build_graph(xml.clone());

    let basis_graph: Graph<BasisNode> = match input_basis_graph {
        Some(graph) => graph,
        None => GraphNode::from_void(),
    };

    absorb(Arc::clone(&basis_graph), Arc::clone(&input_tree));
    log::info!("Done absorbing input tree into basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_absorbed");

    cyclize(Arc::clone(&basis_graph));
    log::info!("Done cyclizing basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_cyclized");

    prune(Arc::clone(&basis_graph));
    log::info!("Done pruning basis graph");
    read_lock!(basis_graph).debug_visualize("basis_graph_pruned");
    read_lock!(basis_graph).debug_statistics("basis_graph_pruned");

    log::info!("Interpreting basis graph...");
    interpret(Arc::clone(&basis_graph), Arc::clone(&output_tree)).await;
    log::info!("Done interpreting basis graph.");
    read_lock!(basis_graph).debug_visualize("basis_graph_interpreted");

    let harvest = Traversal::from_tree(Arc::clone(&output_tree))
        .with_basis(Arc::clone(&basis_graph))
        .harvest()?;

    Ok(NormalizeResult {
        basis_graph: basis_graph,
        harvest: harvest,
    })
}

impl<T: GraphNodeData> GraphNode<T> {
    pub fn serialize(&self) -> SerdeResult<String> {
        let mut visited = HashSet::new();
        let json_value = self.serialize_node(&mut visited)?;
        to_string_pretty(&json_value)
    }

    pub fn deserialize(json_str: &str) -> SerdeResult<Graph<T>> {
        let json_value: JsonValue = from_str(json_str)?;
        let mut visited = HashMap::new();
        Self::deserialize_node(&json_value, &mut visited)
    }

    fn deserialize_node(
        json_value: &JsonValue,
        visited: &mut HashMap<String, Graph<T>>,
    ) -> SerdeResult<Graph<T>> {
        let id = json_value["id"].as_str().unwrap().to_string();

        if let Some(existing_node) = visited.get(&id) {
            return Ok(Arc::clone(existing_node));
        }

        let data: T = serde_json::from_value(json_value["data"].clone())?;

        let temp_node = Arc::new(RwLock::new(GraphNode {
            id: id.clone(),
            hash: json_value["hash"].as_str().unwrap().to_string(),
            data,
            parents: Vec::new(),
            children: Vec::new(),
        }));
        visited.insert(id.clone(), Arc::clone(&temp_node));

        let default_parents = vec![];
        let parents_json = json_value["parents"].as_array().unwrap_or(&default_parents);
        let parents: SerdeResult<Vec<_>> = parents_json
            .iter()
            .map(|parent_json| Self::deserialize_node(parent_json, visited))
            .collect();

        let default_children = vec![];
        let children_json = json_value["children"].as_array().unwrap_or(&default_children);
        let children: SerdeResult<Vec<_>> = children_json
            .iter()
            .map(|child_json| Self::deserialize_node(child_json, visited))
            .collect();

        {
            let mut node = temp_node.write().unwrap();
            node.parents = parents?;
            node.children = children?;
        }

        Ok(temp_node)
    }

    fn serialize_node(&self, visited: &mut HashSet<String>) -> SerdeResult<serde_json::Value> {
        if visited.contains(&self.id) {
            return Ok(json!({"id": self.id, "hash": self.hash }));
        }

        visited.insert(self.id.clone());

        let parents_json: SerdeResult<Vec<_>> = self
            .parents
            .iter()
            .map(|parent| read_lock!(parent).serialize_node(visited))
            .collect();

        let children_json: SerdeResult<Vec<_>> = self
            .children
            .iter()
            .map(|child| read_lock!(child).serialize_node(visited))
            .collect();

        Ok(json!({
            "id": self.id,
            "hash": self.hash,
            "data": self.data,
            "parents": parents_json?,
            "children": children_json?,
        }))
    }
}

pub fn serialize(harvest: Harvest, format: HarvestFormats) -> Result<String, Errors> {
    match format {
        HarvestFormats::JSON => {
            log::info!("Serializing harvest as JSON");

            let serialized = serde_json::to_string(&harvest).expect("Could not serialize output to JSON");

            Ok(serialized)
        },
    }
}
