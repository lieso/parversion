use sha2::{Sha256, Digest};
use crate::models::*;
use crate::utilities;
use crate::llm;
use xmltree::{Element, XMLNode};
use async_recursion::async_recursion;
use sled::Db;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use uuid::Uuid;
use std::fs::OpenOptions;
use std::io::Write;

pub fn build_tree(xml: String) -> Rc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let element = Element::parse(&mut reader).expect("Could not parse XML");

    Node::from_element(&element, None)
}

pub fn update_hashes(tree: Rc<Node>) -> HashSet<String> {
    let mut unique_subtrees: HashSet<String> = HashSet::new();

    post_order_traversal(tree.clone(), &mut |node: &Rc<Node>| {
        node.update_hash();

        if let Some(hash) = node.hash.borrow().clone() {
            unique_subtrees.insert(
                hash.clone()
            );
        }
    });

    log::debug!("{:?}", unique_subtrees);

    unique_subtrees
}

pub fn prune_tree(tree: Rc<Node>, unique_subtrees: &HashSet<String>) {

    let mut subtrees_visited: HashMap<String, bool> = unique_subtrees.iter()
        .map(|value| (value.clone(), false))
        .collect();

    bfs(tree.clone(), &mut |node: &Rc<Node>| {
        if let Some(hash) = node.hash.borrow().clone() {
            let subtree_seen = *subtrees_visited.get(&hash).unwrap_or(&false);

            if subtree_seen {
                log::info!("Pruning node with id: {}", node.id.clone());
                node.remove_from_parent();
            } else {
                subtrees_visited.insert(hash.to_string(), true);
            }
        }
    });
}

pub fn bfs(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    let mut queue = VecDeque::new();
    queue.push_back(node.clone());

    while let Some(current) = queue.pop_front() {
        visit(&current);

        for child in current.children.borrow().iter() {
            queue.push_back(child.clone());
        }
    }
}

pub fn post_order_traversal(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    for child in node.children.borrow().iter() {
        post_order_traversal(child.clone(), visit);
    }

    visit(&node);
}

pub fn level_order_traversal(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    visit(&node);

    for child in node.children.borrow().iter() {
        level_order_traversal(child.clone(), visit);
    }
}

pub fn log_tree(tree: Rc<Node>, title: &str) {

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./debug/trees")
        .expect("Could not open file");

    let divider = std::iter::repeat("*").take(100).collect::<String>();
    let text = format!(
        "\n\n{} {}\n",
        divider,
        title
    );

    writeln!(file, "{}", text).expect("Could not write to file");

    level_order_traversal(tree.clone(), &mut |node: &Rc<Node>| {
        let divider = std::iter::repeat("-").take(50).collect::<String>();
        let text = format!(
            "\nID: {}\nHASH: {}\nXML: {}\nTAG: {}\n",
            node.id,
            node.hash.borrow().clone().unwrap_or(String::from("None")),
            node.xml,
            node.tag
        );

        let text = format!("\n{}{}{}\n", divider, text, divider);

        writeln!(file, "{}", text).expect("Could not write to file");
    });
}

impl Node {
    pub fn from_element(element: &Element, parent: Option<Weak<Node>>) -> Rc<Self> {
        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            parent: parent.unwrap_or_else(Weak::new),
            hash: RefCell::new(None),
            xml: utilities::get_element_xml(&element),
            tag: element.name.clone(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
        });

       let children_nodes: Vec<Rc<Node>> = element.children.iter().filter_map(|child| {
            if let XMLNode::Element(child_element) = child {
                Some(Node::from_element(&child_element, Some(Rc::downgrade(&node))))
            } else {
                None
            }
        }).collect();

        node.children.borrow_mut().extend(children_nodes);

        node
    }

    pub fn remove_from_parent(&self) {
        if let Some(parent) = self.parent.upgrade() {
            parent.children.borrow_mut().retain(|child| {
                child.id != self.id
            });
        }
    }

    pub fn update_hash(&self) {
        if self.children.borrow().is_empty() {
            *self.hash.borrow_mut() = None;
            return;
        }

        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.tag.clone());

        for child in self.children.borrow().iter() {
            if let Some(child_hash) = child.hash.borrow().clone() {
                hasher_items.push(child_hash);
            } else {
                hasher_items.push(child.tag.clone());
            }
        }

        hasher_items.sort();

        for item in hasher_items {
            hasher.update(item);
        }

        let result = format!("{:x}", hasher.finalize());

        *self.hash.borrow_mut() = Some(result.to_string());
    }
}
