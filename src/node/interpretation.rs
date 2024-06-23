use sled::Db;
use std::error::Error;
use bincode::{serialize, deserialize};
use std::rc::{Rc};

use super::{Node, ROOT_NODE_HASH, TEXT_NODE_HASH, node_to_html_with_target_node};
use crate::node_data::{NodeData};
use crate::llm;

pub fn get_root_node(node: Rc<Node>) -> Rc<Node> {
    let mut root_node = node.clone();

    loop {
        let parent_option = {
            let parent_borrow = root_node.parent.borrow();
            parent_borrow.clone()
        };

        match parent_option {
            Some(parent) => root_node = parent,
            None => break,
        }
    }

    root_node
}

impl Node {
    pub async fn interpret_node(&self, db: &Db, output_tree: &Rc<Node>) -> bool {
        log::trace!("In interpret_node");

        if let Some(classical_interpretation) = self.interpret_node_classically() {
            log::info!("Node interpreted classically");
            *self.data.borrow_mut() = classical_interpretation;
            return false;
        }

        let key = &self.xml.to_hash();
        log::debug!("key: {}", key);

        let cache = get_node_data(&db, &key)
            .expect("Could not get node data from database");

        if let Some(cache) = cache {
            log::info!("Cache hit!");
            *self.data.borrow_mut() = cache.clone();
            return false;
        } else {
            log::info!("Cache miss!");

            let surrounding_xml: String = self.node_to_xml_snippet_with_context();

            let llm_result: Vec<NodeData> = llm::xml_to_data(&self.xml, surrounding_xml, Vec::new())
                .await
                .expect("LLM unable to generate node data");

            *self.data.borrow_mut() = llm_result.clone();

            store_node_data(&db, &key, llm_result.clone())
                .expect("Unable to persist node data to database");
        }

        true
    }

    fn node_to_xml_snippet_with_context(&self) -> String {
        log::trace!("In node_to_xml_snippet_with_context");

        let root_node = get_root_node(Rc::new(self.clone()));

        // TODO: this is bad, need to properly skip root node
        let html_node = root_node.children.borrow()[0].clone();

        let document = node_to_html_with_target_node(Rc::clone(&html_node), Rc::new(self.clone()));

        if self.xml.is_text() {
            format!(
                "{}<!--Target node start -->{}<!--Target node end -->{}",
                document.0,
                document.2,
                document.4
            )
        } else {
            format!(
                "{}<!--Target node start -->{}<!--Target node end -->{}{}{}",
                document.0,
                document.1,
                document.2,
                document.3,
                document.4
            )
        }
    }

    fn interpret_node_classically(&self) -> Option<Vec<NodeData>> {
        log::trace!("In interpret_node_classically");

        let attributes = self.xml.get_attributes();

        // * Root node
        if self.hash == ROOT_NODE_HASH {
            log::info!("Node is root node, probably don't need to do anything here");
            return Some(Vec::new());
        }

        // * Elements that contain single attribute "class" and nothing else
        if attributes.len() == 1 && attributes[0] == "class" {
            log::info!("Node only contains single attribute 'class'");
            return Some(Vec::new());
        }

        // * Structural elements
        if self.xml.is_element() && self.is_structural() {
            log::info!("Node is structural, nothing to interpret");
            return Some(Vec::new());
        }

        // * Link elements
        if self.xml.get_element_tag_name() == "link" {
            log::info!("Node represents link HTML element. We ignore these, for now...");
            return Some(Vec::new());
        }

        // * Meta elements
        if self.xml.get_element_tag_name() == "meta" {
            log::info!("Node represents meta HTML element. We ignore these, for now...");
            return Some(Vec::new());
        }

        None
    }
}

fn store_node_data(db: &Db, key: &str, nodes: Vec<NodeData>) -> Result<(), Box<dyn Error>> {
    let serialized_nodes = serialize(&nodes)?;
    db.insert(key, serialized_nodes)?;
    Ok(())
}

fn get_node_data(db: &Db, key: &str) -> Result<Option<Vec<NodeData>>, Box<dyn Error>> {
    match db.get(key)? {
        Some(serialized_nodes) => {
            let nodes_data: Vec<NodeData> = deserialize(&serialized_nodes)?;
            Ok(Some(nodes_data))
        },
        None => Ok(None),
    }
} 
