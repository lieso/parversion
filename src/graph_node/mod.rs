use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::Semaphore;
use tokio::task;
use serde_json::{
    json,
    to_string_pretty,
    Result as SerdeResult,
    Value as JsonValue,
    from_str
};

mod debug;

use crate::xml_node::{XmlNode};
use crate::xml_node;
use crate::basis_node::{BasisNode};
use crate::basis_graph::{BasisGraph};
use crate::constants;
use crate::macros::*;
use crate::graph_node::analysis::*;
use crate::basis_graph::{Subgraph};
use crate::config::{CONFIG};
use crate::utility;

#[derive(Clone, Debug)]
pub struct GraphNode<T: GraphNodeData> {
    pub id: String,
    pub hash: String,
    pub lineage: String,
    pub parents: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub data: T,
}

pub type Graph<T> = Arc<RwLock<GraphNode<T>>>;

pub trait GraphNodeData: Clone + Send + Sync + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {
    fn new(description: String) -> Self;
    fn describe(&self) -> String;
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
            lineage: json_value["lineage"].as_str().unwrap().to_string(),
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
            "lineage": self.lineage,
            "data": self.data,
            "parents": parents_json?,
            "children": children_json?,
        }))
    }
}

pub fn get_depth(node: Graph<XmlNode>) -> usize {
    log::trace!("In get_depth");

    let mut depth = 0;

    let mut current = node;

    loop {
        let parent_nodes = current.read().unwrap().parents.clone();
        if parent_nodes.is_empty() {
            break;
        }

        current = Arc::clone(&parent_nodes[0]);
        depth += 1;
    }

    depth
}




pub fn graph_hash<T: GraphNodeData>(graph: Graph<T>) -> String {
    let mut visited: HashSet<String> = HashSet::new();

    fn compute_hash<T: GraphNodeData>(
        node: Graph<T>,
        visited: &mut HashSet<String>,
    ) -> String {
        let node = node.read().unwrap();

        if visited.contains(&node.id) {
            return "cycle".to_owned();
        }

        let mut hash = Hash::with_items(vec![node.hash.clone()]);

        visited.insert(node.id.clone());

        for child in node.children.iter() {
            hash.push(compute_hash(child.clone(), visited));
        }

        visited.remove(&node.id);

        hash.sort().finalize().to_string()
    }

    compute_hash(graph, &mut visited)
}

pub fn bft<T: GraphNodeData>(graph: Graph<T>, visit: &mut dyn FnMut(Graph<T>) -> bool) {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(graph);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(read_lock!(current).id.clone()) {
            continue;
        }

        if !visit(Arc::clone(&current)) {
            break;
        }

        for child in read_lock!(current).children.iter() {
            queue.push_back(child.clone());
        }
    }
}

