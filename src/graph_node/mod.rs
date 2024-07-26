use serde::{Serialize, Deserialize};
use std::sync::{Arc, Weak, Mutex};
use std::collections::HashMap;
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

pub fn build_immutable_graph(graph: Arc<Mutex<MutableGraph<Xml>>>) -> Arc<ImmutableGraph<Xml>> {
    let mut converted = HashMap::new();

    fn recurse(
        graph: &Arc<Mutex<MutableGraph<Xml>>>,
        converted: &mut HashMap<String, Arc<ImmutableGraph<Xml>>>
    ) -> Arc<ImmutableGraph<Xml>> {
        let graph = graph.lock().unwrap();

        log::debug!("graph.id: {}", graph.id);

        if let Some(existing) = converted.get(&graph.id) {
            return existing.clone();
        }

        let placeholder = Arc::new(ImmutableGraph {
            id: graph.id.clone(),
            hash: graph.hash.clone(),
            parents: Vec::new(),
            children: Vec::new(),
            data: graph.data.clone(),
        });

        converted.insert(graph.id.clone(), placeholder.clone());

        let parents: Vec<Arc<ImmutableGraph<Xml>>> = graph
            .parents
            .iter()
            .map(|parent| recurse(parent, converted))
            .collect();

        let children: Vec<Arc<ImmutableGraph<Xml>>> = graph
            .children
            .iter()
            .map(|child| recurse(child, converted))
            .collect();

        let immutable_graph = Arc::new(ImmutableGraph {
            id: placeholder.id.clone(),
            hash: placeholder.hash.clone(),
            parents,
            children,
            data: placeholder.data.clone(),
        });

        converted.insert(graph.id.clone(), immutable_graph.clone());
        immutable_graph
    }
    
    recurse(&graph, &mut converted)
}

pub fn build_graph(xml: String) -> Arc<Mutex<MutableGraph<Xml>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    MutableGraph::from_xml(&xml, Vec::new())
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
