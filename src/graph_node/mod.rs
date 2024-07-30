use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

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
