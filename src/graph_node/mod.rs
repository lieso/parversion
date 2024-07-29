use serde::{Serialize, Deserialize};
use std::sync::{Arc, Weak, Mutex};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

use crate::xml::{Xml};
use crate::xml;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImmutableGraph<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<ImmutableGraph<T>>>,
    pub children: Vec<Arc<ImmutableGraph<T>>>,
    pub data: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MutableGraph<T> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<Mutex<MutableGraph<T>>>>,
    pub children: Vec<Arc<Mutex<MutableGraph<T>>>>,
    pub data: T,
}

pub enum Graph<T: Send + Sync> {
    Immutable(Arc<ImmutableGraph<T>>),
    Mutable(Arc<Mutex<MutableGraph<T>>>),
}

impl<T: Send + Sync> Graph<T> {
    pub fn as_mutable_ref(&self) -> &Arc<Mutex<MutableGraph<T>>> {
        if let Graph::Mutable(ref m) = *self {
            m
        } else {
            panic!("Expected a mutable graph");
        }
    }

    pub fn as_immutable_ref(&self) -> &Arc<ImmutableGraph<T>> {
        if let Graph::Immutable(ref im) = *self {
            im
        } else {
            panic!("Expected an immutable graph");
        }
    }

    pub fn clone(&self) -> Self {
        match self {
            Graph::Immutable(arc) => Graph::Immutable(arc.clone()),
            Graph::Mutable(arc) => Graph::Mutable(arc.clone()),
        }
    }
}

pub trait GraphNode<T: Send + Sync>: Send + Sync {
    fn get_id(&self) -> &String;
    fn get_children(&self) -> Vec<Graph<T>>;
}

impl<T: Send + Sync> GraphNode<T> for ImmutableGraph<T> {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_children(&self) -> Vec<Graph<T>> {
        self.children.iter().cloned().map(Graph::Immutable).collect()
    }
}

impl<T: Send + Sync> GraphNode<T> for MutableGraph<T> {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_children(&self) -> Vec<Graph<T>> {
        self.children.iter().cloned().map(Graph::Mutable).collect()
    }
}

pub fn build_immutable_graph(graph: Arc<Mutex<MutableGraph<Xml>>>) -> Graph<Xml> {
    unimplemented!()
}

pub fn build_graph(xml: String) -> Graph<Xml> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    let mutable_graph = MutableGraph::from_xml(&xml, Vec::new());
    Graph::Mutable(mutable_graph)
}

impl MutableGraph<Xml> {
    fn from_xml(xml: &Xml, parents: Vec<Arc<Mutex<MutableGraph<Xml>>>>) -> Arc<Mutex<Self>> {
        let node = Arc::new(Mutex::new(MutableGraph {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        let children: Vec<Arc<Mutex<Self>>> = xml
            .get_children()
            .iter()
            .map(|child| {
                MutableGraph::from_xml(child, vec![node.clone()])
            })
        .collect();

        node.lock().unwrap().children.extend(children);

        node
    }
}

pub fn bft<T: Send + Sync>(start: Graph<T>, visit: &mut dyn FnMut(&Graph<T>)) {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        let id = match &current {
            Graph::Immutable(node) => node.get_id().clone(),
            Graph::Mutable(node) => node.lock().unwrap().get_id().clone(),
        };

        if !visited.insert(id.clone()) {
            continue;
        }

        visit(&current);

        let children = match &current {
            Graph::Immutable(node) => node.get_children(),
            Graph::Mutable(node) => node.lock().unwrap().get_children(),
        };

        for child in children {
            queue.push_back(child);
        }
    }
}
