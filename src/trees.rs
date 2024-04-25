use sha2::{Sha256, Digest};
use xmltree::{Element, XMLNode};
use sled::Db;
use std::cell::RefCell;
use std::rc::{Rc};
use uuid::Uuid;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::time::{sleep, Duration};

use crate::models::*;
use crate::utilities;
use crate::llm;
use crate::traversals;

const TEXT_NODE_HASH: String = utilities::hash_text(String::from("text_node"));

pub fn build_tree(xml: String) -> Rc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let element = Element::parse(&mut reader).expect("Could not parse XML");

    Node::from_element(&element, None)
}

pub fn tree_to_xml(tree: Rc<Node>) -> String {
    let element = tree.to_element();

    utilities::element_to_string(&element)
}

pub async fn grow_tree(tree: Rc<Node>) {
    log::trace!("In grow_tree");

    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    let mut nodes: Vec<Rc<Node>> = Vec::new();

    traversals::post_order_traversal(tree.clone(), &mut |node: &Rc<Node>| {
        nodes.push(node.clone());
    });

    log::info!("There are {} nodes to be evaluated", nodes.len());

    for (index, node) in nodes.iter().enumerate() {
        log::info!("Interpreting node #{} out of {}", index + 1, nodes.len());
        node.update_node_data(&db).await;
        node.update_node_data_values();
        node.interpret_node_data(&db).await;
        //sleep(Duration::from_secs(1)).await;
    }
}

pub fn prune_tree(tree: Rc<Node>) {
    log::trace!("In prune_tree");

    traversals::bfs(Rc::clone(&tree), &mut |node: &Rc<Node>| {
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

pub fn merge_nodes(parent: Rc<Node>, nodes: (Rc<Node>, Rc<Node>)) {
    log::trace!("In merge_nodes");

    *nodes.1.parent.borrow_mut() = None;

    for child in nodes.1.children.borrow_mut().iter() {
        *child.parent.borrow_mut() = Some(nodes.0.clone()).into();
        nodes.0.children.borrow_mut().push(child.clone());
    }

    parent.children.borrow_mut().retain(|child| child.id != nodes.1.id);
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

    let mut node_count = 0;

    traversals::bfs(tree.clone(), &mut |node: &Rc<Node>| {
        node_count = node_count + 1;

        let divider = std::iter::repeat("-").take(50).collect::<String>();
        let text = format!(
            "\nID: {}\nHASH: {}\nXML: {}\nSUBTREE HASH: {}\nANCESTOR HASH: {}\nCOMPLEX TYPE NAME: {:?}\n",
            node.id,
            node.hash,
            node.xml,
            node.subtree_hash(),
            node.ancestry_hash(),
            node.complex_type_name
        );

        let mut node_data_text = String::from("");

        for d in node.data.borrow().iter() {
            node_data_text = node_data_text + format!(r##"
                xpath: {},
                name: {},
                is_url: {},
                value: {:?}
            "##, d.xpath, d.name, d.is_url, d.value).as_str();
        }

        let text = format!("\n{}{}{}{}\n", divider, text, node_data_text, divider);

        writeln!(file, "{}", text).expect("Could not write to file");
    });

    writeln!(file, "node count: {}", node_count).expect("Could not write to file");
}

pub fn generate_element_node_hash(tag: String, fields: Vec<String>) -> String {
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
        let hash = utilities::hash_text(tag.clone());

        Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: hash,
            parent: None.into(),
            xml: tag.clone(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
            complex_type_name: RefCell::new(None),
        })
    }

    pub fn from_text(text: &xmltree::Text, parent: Option<Rc<Node>>) => Rc<Self> {
        Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: TEXT_NODE_HASH,
            parent: parent.into(),
            xml: text.as_text(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
            complex_type_name: RefCell::new(None),
        })
    }

    pub fn from_element(element: &Element, parent: Option<Rc<Node>>) -> Rc<Self> {
        let tag = element.name.clone();
        let xml = utilities::get_element_xml(&element);

        let element_fields = element.attributes.keys().cloned().collect();

        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: generate_element_node_hash(tag.clone(), element_fields),
            parent: parent.into(),
            xml: xml,
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
            complex_type_name: RefCell::new(None),
        });

       let children_nodes: Vec<Rc<Node>> = element.children.iter().filter_map(|child| {
            match child {
                XMLNode::Element(child_element) => Some(Node::from_element(&child_element, Some(Rc::clone(&node)))),
                XMLNode::Text(child_text) => Some(Node::from_text(&child_text, Some(Rc::clone(&node)))),
                _ => None,
            }
        }).collect();

        node.children.borrow_mut().extend(children_nodes);

        node
    }

    pub fn ancestry_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.hash.clone());

        if let Some(parent) = self.parent.borrow().as_ref() {
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
        unimplemented!()
        //let mut element = Element::new(&self.tag);

        //for child in self.children.borrow().iter() {
        //    element.children.push(
        //        XMLNode::Element(child.to_element())
        //    );
        //}

        //element
    }
}

impl Node {
    pub async fn update_node_data(&self, db: &Db) {
        log::trace!("In update_node_data");

        let interpret = should_interpret(Rc::clone(&self));

        if !interpret {
            log::info!("Ignoring node");
            *self.data.borrow_mut() = Vec::new();
            return;
        }

        if let Some(node_data) = utilities::get_node_data(&db, &self.hash).expect("Could not get node data from database") {
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

    pub async fn interpret_node_data(&self, db: &Db) {
        log::trace!("In interpret_node_data");

        if self.children.borrow().is_empty() {
            log::info!("Ignoring leaf node");
            *self.complex_type_name.borrow_mut() = None.into();
            return;
        }

        let subtree_hash = &self.subtree_hash();

        if let Some(complex_type) = utilities::get_node_complex_type(&db, subtree_hash).expect("Could not get node complex type from database") {
            log::info!("Cache hit!");
            *self.complex_type_name.borrow_mut() = Some(complex_type.clone());
        } else {
            log::info!("Cache miss!");

            let llm_type_name: String = llm::interpret_node(&Rc::new(self.clone())).await
                .expect("Could not interpret node");

            *self.complex_type_name.borrow_mut() = Some(llm_type_name.clone()).into();

            utilities::store_node_complex_type(&db, subtree_hash, &llm_type_name).expect("Unable to persist complex type to database");
        }
    }
}
