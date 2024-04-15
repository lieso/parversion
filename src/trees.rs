use sha2::{Sha256, Digest};
use crate::models::*;
use crate::utilities;
use crate::llm;
use xmltree::{Element, XMLNode};
use sled::Db;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use uuid::Uuid;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::time::{sleep, Duration};

pub fn build_tree(xml: String) -> Rc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let element = Element::parse(&mut reader).expect("Could not parse XML");

    Node::from_element(&element, None)
}

pub fn tree_to_xml(tree: Rc<Node>) -> String {
    let element = tree.to_element();

    utilities::element_to_string(element)
}

pub async fn grow_tree(tree: Rc<Node>) {
    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    let mut nodes: Vec<Rc<Node>> = Vec::new();

    post_order_traversal(tree.clone(), &mut |node: &Rc<Node>| {
        nodes.push(node.clone());
    });

    log::info!("There are {} nodes to be evaluated", nodes.len());

    for node in nodes.iter() {
        node.update_node_data(&db).await;
        node.update_node_data_values();
        sleep(Duration::from_secs(1)).await;
    }
}

pub fn prune_tree(tree: Rc<Node>, unique_subtrees: &HashSet<String>) {

    let mut subtrees_visited: HashMap<String, bool> = unique_subtrees.iter()
        .map(|value| (value.clone(), false))
        .collect();

    bfs(tree.clone(), &mut |node: &Rc<Node>| {
        if let Some(subtree_hash) = node.subtree_hash.borrow().clone() {
            let subtree_seen = *subtrees_visited.get(&subtree_hash).unwrap_or(&false);

            if subtree_seen {
                log::info!("Pruning node with id: {}", node.id.clone());
                node.remove_from_parent();
            } else {
                subtrees_visited.insert(subtree_hash.to_string(), true);
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

pub fn dfs(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    visit(&node);

    for child in node.children.borrow().iter() {
        dfs(child.clone(), visit);
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

    let xml = tree_to_xml(tree.clone());
    let xml_file_name = format!("tree_{}.xml", tree.ancestory_hash);


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
            node.subtree_hash.borrow().clone().unwrap_or(String::from("None")),
            node.xml,
            node.tag
        );
        let text = format!("\n{}{}{}\n", divider, text, divider);

        writeln!(file, "{}", text).expect("Could not write to file");
    });
}

impl Node {
    pub fn from_void() -> Rc<Self> {
        Rc:new(Node {
            id: Uuid::new_v4().to_string(),
            parent: Weak::new,
            subtree_hash: RefCell::new(None),
            ancestry_hash: utilities::hash_text("<>"),
            xml: xml,
            xml_hash: xml_hash,
            tag: tag,
            interpret: false,
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
        })
    }

    pub fn from_element(element: &Element, parent: Option<Weak<Node>>) -> Rc<Self> {
        let tag = element.name.clone();
        let xml = utilities::get_element_xml(&element);
        let xml_hash = utilities::hash_text(&xml);

        let mut parent_contribution = String::from("<>");
        
        if let Some(weak_parent) = parent {
            if let Some(parent) = weak_parent.upgrade() {
                parent_contribution = parent.ancestry_hash;
            }
        }
        let ancestry_hash = utilities::hash_text(
            format!("{}{}", parent_contribution, tag)
        );

        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            parent: parent.unwrap_or_else(Weak::new),
            ancestry_hash: ancestry_hash,
            xml: xml,
            xml_hash: xml_hash,
            tag: tag,
            interpret: element.attributes.len() > 0,
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
        node.subtree_hash = self.generate_subtree_hash();

        node
    }

    pub fn generate_subtree_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.tag.clone());

        for child in self.children.borrow().iter() {
            hasher_items.push(child.subtree_hash);
        }

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    pub fn to_element(&self) -> Element {
        let mut element = Element::new(&self.tag);

        for child in self.children.borrow().iter() {
            element.children.push(
                XMLNode::Element(child.to_element())
            );
        }

        element
    }

    pub fn remove_from_parent(&self) {
        if let Some(parent) = self.parent.upgrade() {
            parent.children.borrow_mut().retain(|child| {
                child.id != self.id
            });
        }
    }
}

impl Node {
    pub async fn update_node_data(&self, db: &Db) {
        log::trace!("In update_node_data");

        if !self.interpret {
            log::info!("Ignoring node");
            *self.data.borrow_mut() = Vec::new();
            return;
        }

        if let Some(node_data) = utilities::get_node_data(&db, &self.xml_hash).expect("Could not update node data") {
            log::info!("Cache hit!");
            *self.data.borrow_mut() = node_data.clone();
        } else {
            log::info!("Cache miss!");
            let llm_node_data: Vec<NodeData> = llm::generate_node_data(self.xml.clone()).await.expect("LLM unable to generate node data");
            *self.data.borrow_mut() = llm_node_data.clone();

            utilities::store_node_data(&db, &self.xml_hash, llm_node_data.clone()).expect("Unable to persist node data to database");
        }
    }

    pub fn update_node_data_values(&self) {
        let mut data = self.data.borrow_mut();

        for item in data.iter_mut() {
            if let Ok(result) = utilities::apply_xpath(&self.xml, &item.xpath) {
                log::trace!("xpath success match: {}", result);
                item.value = Some(result.clone());
            } else {
                log::warn!("Could not apply xpath: {} to node with id: {}", &item.xpath, self.id);
                item.value = None;
            }
        }
    }
}


