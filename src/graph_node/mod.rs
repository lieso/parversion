use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use sha2::{Sha256, Digest};

use crate::xml_node::{XmlNode};
use crate::xml_node;
use crate::basis_node::{BasisNode};
use crate::constants;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode<T: GraphNodeData> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub data: T,
}

pub type Graph<T> = Arc<RwLock<GraphNode<T>>>;

pub trait GraphNodeData {
    fn new() -> Self;
}

pub fn build_graph(xml: String) -> Arc<RwLock<GraphNode<XmlNode>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = XmlNode::parse(&mut reader).expect("Could not parse XML");

    GraphNode::from_xml(&xml, Vec::new())
}

impl GraphNode<XmlNode> {
    fn from_xml(xml: &XmlNode, parents: Vec<Graph<XmlNode>>) -> Graph<XmlNode> {
        let node = Arc::new(RwLock::new(GraphNode {
            id: Uuid::new_v4().to_string(),
            hash: xml_node::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        {
            let children: Vec<Graph<XmlNode>> = xml
                .get_children()
                .iter()
                .map(|child| {
                    GraphNode::from_xml(child, vec![node.clone()])
                })
                .collect();

            let mut node_write_lock = node.write().unwrap();
            node_write_lock.children.extend(children);
        }

        node
    }
}

impl<T: GraphNodeData> GraphNode<T> {
    pub fn from_void() -> Graph<T> {
        Arc::new(RwLock::new(GraphNode {
            id: Uuid::new_v4().to_string(),
            hash: constants::ROOT_NODE_HASH.to_string(),
            parents: Vec::new(),
            children: Vec::new(),
            data: T::new(),
        }))
    }
}

pub fn subgraph_hash<T: GraphNodeData>(graph: Graph<T>) -> String {
    let mut visited: HashSet<String> = HashSet::new();

    fn compute_hash<T: GraphNodeData>(
        node: Graph<T>,
        visited: &mut HashSet<String>,
    ) -> String {
        let node = node.read().unwrap();

        if visited.contains(&node.id) {
            return "cycle".to_owned();
        }

        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(node.hash.clone());

        visited.insert(node.id.clone());

        for child in node.children.iter() {
            hasher_items.push(compute_hash(child.clone(), visited));
        }

        visited.remove(&node.id);

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    compute_hash(graph, &mut visited)
}

pub fn deep_copy<T: GraphNodeData, U: GraphNodeData>(graph: Graph<U>, parents: Vec<Graph<T>>) -> Graph<T> where T: GraphNodeData {
    log::trace!("In deep_copy");

    let guard = graph.read().unwrap();

    let new_node = Arc::new(RwLock::new(GraphNode {
        id: guard.id.clone(),
        hash: guard.hash.clone(),
        parents,
        children: Vec::new(),
        data: T::new(),
    }));

    {
        let children: Vec<Graph<T>> = guard.children.iter()
            .map(|child| deep_copy(child.clone(), vec![new_node.clone()]))
            .collect();
        let mut node_write_lock = new_node.write().unwrap();
        node_write_lock.children.extend(children);
    }

    new_node
}

pub fn absorb<T: GraphNodeData, U: GraphNodeData>(recipient: Graph<T>, donor: Graph<U>) {
    log::trace!("In absorb");

    let recipient_child = {
        recipient
            .read()
            .unwrap()
            .children
            .iter()
            .find(|item| {
                item.read().unwrap().hash == donor.read().unwrap().hash
            })
            .cloned()
    };

    if let Some(recipient_child) = recipient_child {
        log::trace!("Donor and recipient node have the same hash");

        if subgraph_hash(recipient_child.clone()) != subgraph_hash(donor.clone()) {
            log::trace!("Donor and recipient child have differing subgraph hashes");
            let donor_children = donor.read().unwrap().children.clone();

            for donor_child in donor_children.iter() {
                absorb(recipient_child.clone(), donor_child.clone());
            }
        }
    } else {
        log::trace!("Donor and recipient subgraphs incompatible. Adopting donor node...");

        let copied = deep_copy::<T, U>(donor, vec![recipient.clone()]);
        recipient.write().unwrap().children.push(copied.clone());
    }
}

pub fn bft<T: GraphNodeData>(graph: Graph<T>, visit: &mut dyn FnMut(Graph<T>)) {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(graph);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.read().unwrap().id.clone()) {
            continue;
        }

        visit(Arc::clone(&current));

        for child in current.read().unwrap().children.iter() {
            queue.push_back(child.clone());
        }
    }
}
