use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use tokio::sync::Semaphore;
use tokio::task;

mod debug;
mod analysis;

use crate::xml_node::{XmlNode};
use crate::xml_node;
use crate::basis_node::{BasisNode};
use crate::constants;
use crate::macros::*;
use crate::graph_node::analysis::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode<T: GraphNodeData> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub data: T,
}

pub type Graph<T> = Arc<RwLock<GraphNode<T>>>;

pub trait GraphNodeData: Clone + Send + Sync {
    fn new(description: String) -> Self;
    fn describe(&self) -> String;
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
            data: T::new("blank".to_string()),
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
        data: T::new(guard.data.describe()),
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

pub fn cyclize<T: GraphNodeData>(graph: Graph<T>) {
    log::trace!("In cyclize");

    fn dfs<T: GraphNodeData>(
        node: Graph<T>,
        visited: &mut HashMap<String, Graph<T>>
    ) {
        let node_id = node.read().unwrap().id.clone();
        let node_hash = node.read().unwrap().hash.clone();

        if let Some(first_occurrence) = visited.get(&node_hash) {
            log::info!("Detected cycle");

            for parent in node.read().unwrap().parents.iter() {
                let mut write_lock = parent.write().unwrap();
                write_lock.children.retain(|child| child.read().unwrap().id != node_id);
                write_lock.children.push(first_occurrence.clone());
            }

            let children = node.write().unwrap().children.drain(..).collect::<Vec<_>>();
            for child in children {
                let mut write_lock = child.write().unwrap();
                write_lock.parents.retain(|parent| parent.read().unwrap().id != node_id);
                write_lock.parents.push(first_occurrence.clone());

                first_occurrence.write().unwrap().children.push(child.clone());
            }
        } else {
            visited.insert(node_hash.clone(), Arc::clone(&node));

            let children: Vec<Graph<T>>;
            {
                let read_lock = node.read().unwrap();
                children = read_lock.children.clone();
            }

            for child in children.iter() {
                dfs(
                    child.clone(),
                    visited,
                );
            }

            visited.remove(&node_hash);
        }
    }

    dfs(
        graph,
        &mut HashMap::new(),
    );
}

pub fn prune<T: GraphNodeData>(graph: Graph<T>) {
    log::trace!("In prune");

    fn is_twin<T: GraphNodeData>(a: Graph<T>, b: Graph<T>) -> bool {
        let a_rl = a.read().unwrap();
        let b_rl = b.read().unwrap();
        a_rl.id != b_rl.id && a_rl.hash == b_rl.hash
    }

    bft(Arc::clone(&graph), &mut |parent: Graph<T>| {
        loop {
            let children: Vec<Graph<T>>;
            {
                let read_lock = parent.read().unwrap();
                children = read_lock.children.clone();
            }

            let maybe_twins: Option<(Graph<T>, Graph<T>)> = children
                .iter()
                .find_map(|child| {
                    children
                        .iter()
                        .find(|sibling| is_twin(Arc::clone(child), Arc::clone(sibling)))
                        .map(|sibling| (Arc::clone(child), Arc::clone(sibling)))
                });

            if let Some(twins) = maybe_twins {
                log::info!("Found two siblings nodes that are twins");
                merge_nodes(Arc::clone(&parent), twins);
            } else {
                break;
            }
        }
    });
}

pub async fn interpret<T: GraphNodeData + 'static>(graph: Graph<T>, output_tree: Graph<XmlNode>) {
    log::trace!("In interpret");

    let mut nodes: Vec<Graph<T>> = Vec::new();

    bft(Arc::clone(&graph), &mut |node: Graph<T>| {
        nodes.push(node.clone());
    });

    let semaphore = Arc::new(Semaphore::new(constants::MAX_CONCURRENCY));
    let mut handles = vec![];

    for node in nodes.iter() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(task::spawn(analyze_structure(Arc::clone(node), Arc::clone(&output_tree), permit)));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

fn merge_nodes<T: GraphNodeData>(parent: Graph<T>, nodes: (Graph<T>, Graph<T>)) {
    log::trace!("In merge_nodes");

    let keep_node = nodes.0;
    let discard_node = nodes.1;

    {
        discard_node.write().unwrap().parents.clear();
    }

    for child in discard_node.read().unwrap().children.iter() {
        let mut write_lock = child.write().unwrap();
        write_lock.parents.retain(|p| p.read().unwrap().id != discard_node.read().unwrap().id);
        write_lock.parents.push(Arc::clone(&keep_node));

        keep_node.write().unwrap().children.push(Arc::clone(child));
    }

    parent.write().unwrap().children.retain(|child| child.read().unwrap().id != discard_node.read().unwrap().id);
}
