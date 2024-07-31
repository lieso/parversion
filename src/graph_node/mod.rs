use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use sha2::{Sha256, Digest};

use crate::xml::{Xml};
use crate::xml;
use crate::basis_node::{BasisNode};
use crate::constants;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MutexGraphNode<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<Mutex<MutexGraphNode<T>>>>,
    pub children: Vec<Arc<Mutex<MutexGraphNode<T>>>>,
    pub data: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RwLockGraphNode<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<RwLock<RwLockGraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<RwLockGraphNode<T>>>>,
    pub data: T,
}

pub type MutexGraph<T> = Arc<Mutex<MutexGraphNode<T>>>;
pub type RwLockGraph<T> = Arc<RwLock<RwLockGraphNode<T>>>;

pub trait GraphNodeData {
    fn new() -> Self;
}

pub fn build_rwlock_graph(xml: String) -> Arc<RwLock<RwLockGraphNode<Xml>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    RwLockGraphNode::from_xml(&xml, Vec::new())
}

impl RwLockGraphNode<Xml> {
    fn from_xml(xml: &Xml, parents: Vec<Arc<RwLock<RwLockGraphNode<Xml>>>>) -> Arc<RwLock<RwLockGraphNode<Xml>>> {
        let node = Arc::new(RwLock::new(RwLockGraphNode {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        {
            let children: Vec<Arc<RwLock<RwLockGraphNode<Xml>>>> = xml
                .get_children()
                .iter()
                .map(|child| {
                    RwLockGraphNode::from_xml(child, vec![node.clone()])
                })
                .collect();

            let mut node_write_lock = node.write().unwrap();
            node_write_lock.children.extend(children);
        }

        node
    }
}

pub fn build_mutex_graph(xml: String) -> Arc<Mutex<MutexGraphNode<Xml>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    MutexGraphNode::from_xml(&xml, Vec::new())
}

impl MutexGraphNode<Xml> {
    fn from_xml(xml: &Xml, parents: Vec<Arc<Mutex<MutexGraphNode<Xml>>>>) -> Arc<Mutex<MutexGraphNode<Xml>>> {
        let node = Arc::new(Mutex::new(MutexGraphNode {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        let children: Vec<Arc<Mutex<MutexGraphNode<Xml>>>> = xml
            .get_children()
            .iter()
            .map(|child| {
                MutexGraphNode::from_xml(child, vec![node.clone()])
            })
            .collect();

        node.lock().unwrap().children.extend(children);

        node
    }
}

impl MutexGraphNode<BasisNode> {
    pub fn from_void() -> MutexGraph<BasisNode> {
        Arc::new(Mutex::new(MutexGraphNode {
            id: Uuid::new_v4().to_string(),
            hash: constants::ROOT_NODE_HASH.to_string(),
            parents: Vec::new(),
            children: Vec::new(),
            data: BasisNode {
                data: Arc::new(Mutex::new(Vec::new())),
                structure: Arc::new(Mutex::new(Vec::new())),
            },
        }))
    }
}

pub fn subgraph_hash<T>(graph: MutexGraph<T>) -> String {
    let mut visited: HashSet<String> = HashSet::new();

    fn compute<T>(
        node: MutexGraph<T>,
        visited: &mut HashSet<String>,
    ) -> String {
        let node = node.lock().unwrap();

        if visited.contains(&node.id) {
            return "cycle".to_owned();
        }

        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(node.hash.clone());

        visited.insert(node.id.clone());

        for child in node.children.iter() {
            hasher_items.push(compute(child.clone(), visited));
        }

        visited.remove(&node.id);

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    compute(graph, &mut visited)
}

pub fn deep_copy<T: GraphNodeData, U>(graph: MutexGraph<U>, parents: Vec<MutexGraph<T>>) -> MutexGraph<T> where T: GraphNodeData {
    log::trace!("In deep_copy");

    let guard = graph.lock().unwrap();

    let new_node = Arc::new(Mutex::new(MutexGraphNode {
        id: guard.id.clone(),
        hash: guard.hash.clone(),
        parents,
        children: Vec::new(),
        data: T::new(),
    }));

    let children: Vec<MutexGraph<T>> = guard.children.iter()
        .map(|child| deep_copy(child.clone(), vec![new_node.clone()]))
        .collect();
    new_node.lock().unwrap().children.extend(children);

    new_node
}

pub fn absorb<T: GraphNodeData, U>(recipient: MutexGraph<T>, donor: MutexGraph<U>) {
    log::trace!("In absorb");

    let recipient_child = {
        recipient
            .lock()
            .unwrap()
            .children
            .iter()
            .find(|item| {
                item.lock().unwrap().hash == donor.lock().unwrap().hash
            })
            .cloned()
    };

    if let Some(recipient_child) = recipient_child {
        log::trace!("Donor and recipient node have the same hash");

        if subgraph_hash(recipient_child.clone()) != subgraph_hash(donor.clone()) {
            log::trace!("Donor and recipient child have differing subgraph hashes");
            let donor_children = donor.lock().unwrap().children.clone();

            for donor_child in donor_children.iter() {
                absorb(recipient_child.clone(), donor_child.clone());
            }
        }
    } else {
        log::trace!("Donor and recipient subgraphs incompatible. Adopting donor node...");

        let typed_donor = deep_copy::<T, U>(donor, vec![recipient.clone()]);

        //typed_donor.lock().unwrap().parents = vec![recipient.clone()];
        recipient.lock().unwrap().children.push(typed_donor.clone());
    }
}
