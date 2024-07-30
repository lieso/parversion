use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

use crate::xml::{Xml};
use crate::xml;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MutexGraph<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<Mutex<MutexGraph<T>>>>,
    pub children: Vec<Arc<Mutex<MutexGraph<T>>>>,
    pub data: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RwLockGraph<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<RwLock<RwLockGraph<T>>>>,
    pub children: Vec<Arc<RwLock<RwLockGraph<T>>>>,
    pub data: T,
}

pub fn build_rwlock_graph(xml: String) -> Arc<RwLock<RwLockGraph<Xml>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    RwLockGraph::from_xml(&xml, Vec::new())
}

impl RwLockGraph<Xml> {
    fn from_xml(xml: &Xml, parents: Vec<Arc<RwLock<RwLockGraph<Xml>>>>) -> Arc<RwLock<RwLockGraph<Xml>>> {
        let node = Arc::new(RwLock::new(RwLockGraph {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        {
            let children: Vec<Arc<RwLock<RwLockGraph<Xml>>>> = xml
                .get_children()
                .iter()
                .map(|child| {
                    RwLockGraph::from_xml(child, vec![node.clone()])
                })
                .collect();

            let mut node_write_lock = node.write().unwrap();
            node_write_lock.children.extend(children);
        }

        node
    }
}

pub fn build_mutex_graph(xml: String) -> Arc<Mutex<MutexGraph<Xml>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    MutexGraph::from_xml(&xml, Vec::new())
}

impl MutexGraph<Xml> {
    fn from_xml(xml: &Xml, parents: Vec<Arc<Mutex<MutexGraph<Xml>>>>) -> Arc<Mutex<MutexGraph<Xml>>> {
        let node = Arc::new(Mutex::new(MutexGraph {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        let children: Vec<Arc<Mutex<MutexGraph<Xml>>>> = xml
            .get_children()
            .iter()
            .map(|child| {
                MutexGraph::from_xml(child, vec![node.clone()])
            })
            .collect();

        node.lock().unwrap().children.extend(children);

        node
    }
}
