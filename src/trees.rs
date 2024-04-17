use sha2::{Sha256, Digest};
use crate::models::*;
use crate::utilities;
use crate::llm;
use xmltree::{Element, XMLNode};
use sled::Db;
use std::collections::{VecDeque, HashMap, HashSet};
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

pub fn prune_tree(tree: Rc<Node>) {
    traversal::bfs(Rc::clone(tree), &mut |node: &Rc<Node>| {
        loop {
            let twins: Option<(Rc<Node>, Rc<Node>)> = tree.children.borrow().iter()
                .try_fold(None, |acc, child| {

                    if acc.is_some() Err(acc);

                    if let Some(sibling) = tree.children.find(|c| {
                        c.id != child.id && c.hash == child.hash
                    }) {
                        Some((child, sibling))
                    } else {
                        None
                    }
                })
                .map_or_else(|opt1| opt1, |opt2| opt2);

            if let Some(twins) = twins {
                merge_nodes(twins);
            } else {
                break;
            }
        }
    });
}

pub fn merge_nodes((Rc<Node>, Rc<Node>)) {
    unimplemented!()
}

pub fn absorb_tree(recipient: Rc<Node>, donor: Rc<Node>) {

   if let Some(recipient_child) = recipient.children.find(|item| item.hash == donor.hash) {
        if recipient_child.subtree_hash() == donor.subtree_hash() {
            return;
        } else {
            for donor_child in donor.children.borrow_mut().iter() {
                absorb_tree(recipient_child, donor_child.clone());
            }
        }
    } else {
        recipient.adopt_child(donor);
    }
}

pub fn log_tree(tree: Rc<Node>, title: &str) {

    let xml = tree_to_xml(tree.clone());
    let xml_file_name = format!("tree_{}.xml", tree.ancestry_hash());


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

    dfs(tree.clone(), &mut |node: &Rc<Node>| {
        let divider = std::iter::repeat("-").take(50).collect::<String>();
        let text = format!(
            "\nID: {}\nHASH: {}\nXML: {}\nTAG: {}\n",
            node.id,
            node.subtree_hash(),
            node.xml,
            node.tag
        );
        let text = format!("\n{}{}{}\n", divider, text, divider);

        writeln!(file, "{}", text).expect("Could not write to file");
    });
}

pub fn generate_node_hash(tag: String, fields: Vec<String>) -> String {
    let mut hasher = Sha256::new();
    
    let mut hasher_items = Vec::new();
    hasher_items.push(tag);

    for field in fields.iter() {
        hasher_items.push(field.to_string());
    }

    hasher_items.sort();

    hasher.update(hasher_items.join(""));

    format!("{:x}", hasher.finalize())
}

impl Node {
    pub fn from_void() -> Rc<Self> {
        let tag = String::from("<>");
        let hash = utilities::hash_text(tag);

        Rc:new(Node {
            id: Uuid::new_v4().to_string(),
            hash: hash,
            parent: Weak::new(),
            xml: tag,
            tag: tag,
            interpret: false,
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
        })
    }

    pub fn from_element(element: &Element, parent: Option<Weak<Node>>) -> Rc<Self> {
        let tag = element.name.clone();
        let xml = utilities::get_element_xml(&element);

        let element_fields = element.attributes.keys().cloned().collect();

        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: generate_node_hash(tag.clone(), element_fields),
            parent: parent.unwrap_or_else(Weak::new),
            xml: xml,
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

        node
    }

    pub fn ancestry_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.hash.clone());

        if let Some(parent) = &self.parent.upgrade() {
            hasher_items.push(
                parent.ancestry_hash()
            );
        }

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    pub fn subtree_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.hash.clone());

        for child in self.children.borrow().iter() {
            hasher_items.push(child.subtree_hash());
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
    
    pub fn get_lineage(&self) -> VecDeque<&str> {
        if let Some(parent) = self.parent.upgrade() {
            let lineage = parent.get_lineage().clone();
            lineage.push_back(&self.hash.clone());
            return lineage;
        } else {
            VecDeque::new()
        }
    }

    pub fn adopt_child(self, tree: Rc<Node>) {
        tree.parent = Rc::downgrade(&Rc<self>);
        self.children.borrow_mut().push(tree);
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

        if let Some(node_data) = utilities::get_node_data(&db, &self.hash).expect("Could not update node data") {
            log::info!("Cache hit!");
            *self.data.borrow_mut() = node_data.clone();
        } else {
            log::info!("Cache miss!");
            let llm_node_data: Vec<NodeData> = llm::generate_node_data(self.xml.clone()).await.expect("LLM unable to generate node data");
            *self.data.borrow_mut() = llm_node_data.clone();

            utilities::store_node_data(&db, &self.hash, llm_node_data.clone()).expect("Unable to persist node data to database");
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


