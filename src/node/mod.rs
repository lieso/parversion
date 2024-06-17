use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tokio::time::{sleep, Duration};

use std::rc::{Rc};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

mod debug;
mod interpretation;
mod traversal;
mod utility;

use crate::node_data::{NodeData};
use crate::node::traversal::*;
use crate::xml::*;

// echo -n "text" | sha256sum
const TEXT_NODE_HASH: &str = "982d9e3eb996f559e633f4d194def3761d909f5a3b647d1a851fead67c32c9d1";
// echo -n "root" | sha256sum
const ROOT_NODE_HASH: &str = "4813494d137e1631bba301d5acab6e7bb7aa74ce1185d456565ef51d737677b2";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tree {
    pub root: Rc<Node>, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub id: String,
    pub hash: String,
    pub xml: Xml,
    pub parent: RefCell<Option<Rc<Node>>>,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
    pub minor: RefCell<Option<Rc<Node>>>,
    pub complex_type_name: RefCell<Option<String>>,
}

impl Node {
    pub fn from_void() -> Rc<Self> {
        Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: ROOT_NODE_HASH.to_string(),
            xml: Xml::from_void(),
            parent: None.into(),
            minor: None.into(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
            complex_type_name: RefCell::new(None),
        })
    }

    pub fn from_xml(xml: &Xml, parent: Option<Rc<Node>>) -> Rc<Self> {
        if xml.is_text() {
            return Rc::new(Node {
                id: Uuid::new_v4().to_string(),
                hash: TEXT_NODE_HASH.to_string(),
                xml: xml.without_children(),
                parent: parent.into(),
                data: RefCell::new(Vec::new()),
                children: RefCell::new(vec![]),
                minor: None.into(),
                complex_type_name: RefCell::new(None),
            })
        }

        let tag = xml.get_element_tag_name();
        let attributes = xml.get_attributes();

        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: utility::generate_element_node_hash(vec![tag.clone()], attributes),
            xml: xml.without_children(),
            parent: parent.into(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
            minor: None.into(),
            complex_type_name: RefCell::new(None),
        });

        let children: Vec<Rc<Node>> = xml.get_children().iter().map(|child| {
            Node::from_xml(child, Some(Rc::clone(&node)))
        }).collect();

        node.children.borrow_mut().extend(children);

        node
    }
}

pub fn build_tree(xml: String) -> Rc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    Node::from_xml(&xml, None)
}

pub async fn grow_tree(tree: Rc<Node>) {
    log::trace!("In grow_tree");

    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    let mut nodes: Vec<Rc<Node>> = Vec::new();

    post_order_traversal(tree.clone(), &mut |node: &Rc<Node>| {
        nodes.push(node.clone());
    });

    log::info!("There are {} nodes to be evaluated", nodes.len());

    for (index, node) in nodes.iter().enumerate() {
        log::info!("--- Analysing node #{} out of {} ---", index + 1, nodes.len());
        log::debug!("id: {}, xml: {}", node.id, node.xml);

        if node.hash == ROOT_NODE_HASH {
            log::info!("Node is root node, probably don't need to do anything here");
            continue;
        }

        if node.update_node_data(&db).await {
            sleep(Duration::from_secs(1)).await;
        }

        node.update_node_data_values();

        if node.interpret_node_data(&db).await {
            sleep(Duration::from_secs(1)).await;
        }
    }
}

pub fn prune_tree(tree: Rc<Node>) {
    log::trace!("In prune_tree");

    bfs(Rc::clone(&tree), &mut |node: &Rc<Node>| {
        loop {
            if node.parent.borrow().is_none() {
                break;
            }

            let mut children_borrow = node.children.borrow();
            log::debug!("Node has {} children", children_borrow.len());
            
            let twins: Option<(Rc<Node>, Rc<Node>)> = children_borrow.iter()
                .find_map(|child| {
                    children_borrow.iter()
                        .find(|&sibling| sibling.id != child.id && sibling.hash == child.hash && sibling.parent.borrow().is_some())
                        .map(|sibling| (Rc::clone(child), Rc::clone(sibling)))
                });

            drop(children_borrow);

            if let Some(twins) = twins {
                log::trace!("Pruning nodes with ids: {} and {} with hash {}", twins.0.id, twins.1.id, twins.0.hash);
                merge_nodes(node.clone(), twins);
            } else {
                break;
            }
        }
    });
}

pub fn absorb_tree(recipient: Rc<Node>, donor: Rc<Node>) {
    log::trace!("In absorb_tree");

    let recipient_child = {
        recipient.children.borrow().iter().find(|item| item.hash == donor.hash).cloned()
    };

   if let Some(recipient_child) = recipient_child {
       log::trace!("Donor and recipient node have the same hash");

       if recipient_child.subtree_hash() == donor.subtree_hash() {
           log::trace!("Donor and recipient child subtree hashes match");
           return;
       } else {
           log::trace!("Donor and recipient child have differing subtree hashes");
           let donor_children = donor.children.borrow().clone();

           for donor_child in donor_children.iter() {
               absorb_tree(recipient_child.clone(), donor_child.clone());
           }
       }
    } else {
        log::trace!("Donor and recipient subtrees incompatible. Adopting donor node...");

        //recipient.adopt_child(donor);

        *donor.parent.borrow_mut() = Some(recipient.clone());
        recipient.children.borrow_mut().push(donor);
    }
}

pub fn search_tree_by_lineage(mut tree: Rc<Node>, mut lineage: VecDeque<String>) -> Option<Rc<Node>> {
    log::trace!("In search_tree_by_lineage");

    while let Some(hash) = lineage.pop_front() {
        log::trace!("hash: {}", hash);

        let node = tree
            .children
            .borrow()
            .iter()
            .find(|item| item.hash == hash)
            .cloned();

        if let Some(node) = node {
            tree = node;
        } else {
            log::info!("Could not find child node with hash");

            let find_minor = |n: &Rc<Node>| {
                fn recurse(n: &Rc<Node>, hash: &str) -> Option<Rc<Node>> {
                    if let Some(minor) = n.minor.borrow().as_ref() {
                        log::debug!("{} x {}", minor.hash, hash);
                        if minor.hash == hash {
                            return Some(minor.clone());
                        } else {
                            return recurse(minor, hash);
                        }
                    }

                    None
                }
                recurse(n, &hash)
            };

            if find_minor(&tree).is_none() {
                return None;
            }
        }
    }

    Some(tree)
}

pub fn node_data_to_hash_map(node_data: &RefCell<Vec<NodeData>>, output_tree: Rc<Node>) -> HashMap<String, HashMap<String, String>> {
    log::trace!("In node_data_to_hash_map");

    let mut values: HashMap<String, HashMap<String, String>> = HashMap::new();

    for item in node_data.borrow().iter() {
        if let Some(node_data_value) = item.select(output_tree.xml.clone()) {

            let mut value = HashMap::new();
            value.insert(String::from("value"), node_data_value.text.clone());
            value.insert(String::from("is_url"), item.is_url.to_string());
            value.insert(String::from("is_id"), item.is_id.to_string());
            value.insert(String::from("is_decorative"), item.is_decorative.to_string());
            value.insert(String::from("is_js"), item.is_js.to_string());

            values.insert(item.name.clone(), value);

        } else {
            log::warn!("Basis tree node could not be applied to output tree node!");
        }
    }

    values
}

fn merge_nodes(parent: Rc<Node>, nodes: (Rc<Node>, Rc<Node>)) {
    log::trace!("In merge_nodes");

    *nodes.1.parent.borrow_mut() = None;

    for child in nodes.1.children.borrow_mut().iter() {
        *child.parent.borrow_mut() = Some(nodes.0.clone()).into();
        nodes.0.children.borrow_mut().push(child.clone());
    }

    parent.children.borrow_mut().retain(|child| child.id != nodes.1.id);
}
