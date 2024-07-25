use serde::{Serialize, Deserialize};
use std::sync::{Arc, Weak, Mutex};
use uuid::Uuid;

use crate::xml::{Xml};
use crate::xml;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode<T, U> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<U<Weak<GraphNode<T, U>>>>,
    pub children: Vec<U<GraphNode<T, U>>>,
    pub data: T,
}

pub fn build_immutable_graph<T>(xml: String) -> Arc<GraphNode<T, Arc>> {
    let graph = build_graph::<T>(xml);
    GraphNode::freeze(graph)
}

pub fn build_graph<T>(xml: String) -> Arc<Mutex<GraphNode<T, Arc<Mutex>>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    GraphNode::from_xml(&xml, Vec::new())
}

impl<T> GraphNode<T, Arc<Mutex>> {
    fn from_xml(xml: &Xml, parents: Vec<Weak<Mutex<Self>>>) -> Arc<Mutex<Self>> {
        let node = Arc::new(Mutex::new(GraphNode {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(&xml),
            parents: parents.into(),
            children: Vec::new(),
            data: T::xml_to_data(&xml),
        }));

        let weak_node = Arc::downgrade(&node);

        let children: Vec<Arc<Mutex<Self>>> = xml
            .get_children()
            .iter()
            .map(|child| GraphNode::from_xml(child, vec![weak_node.clone()]))
            .collect();

        node.lock().unwrap().children.extend(children);

        node
    }

    fn freeze(graph: Arc<Mutex<GraphNode<T, Arc<Mutex>>>>) -> Arc<GraphNode<T, Arc>> {
        let graph = graph.lock().unwrap();

        fn recurse<T>(
            node: &GraphNode<T, Arc<Mutex>>,
        ) -> Arc<GraphNode<T, Arc>> {
            let new_children: Vec<Arc<GraphNode<T, Arc>>> = node
                .children
                .iter()
                .map(|child| {
                    let locked_child = child.lock().unwrap();
                    recurse(&*locked_child)
                })
                .collect();

            let new_parents: Vec<Weak<GraphNode<T, Arc>>> = node
                .parents
                .iter()
                .map(|weak_parent| {
                    weak_parent.upgrade().map(|arc_mutex_parent| {
                        Arc::downgrade(&recurse(&arc_mutex_parent.lock().unwrap()))
                    }).unwrap()
                })
                .collect();

            Arc::new(GraphNode {
                id: node.id.clone(),
                hash: node.hash.clone(),
                parents: new_parents,
                children: new_children,
                data: node.data.clone(),
            })
        }

        recurse(&graph)
    }
}
