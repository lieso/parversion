use sled::Db;
use std::error::Error;
use bincode::{serialize, deserialize};
use std::rc::{Rc};

use super::{Node, node_to_html_with_target_node, find_all_node_xml_by_lineage};
use crate::node_data::{NodeData};
use crate::llm;
use crate::config::{CONFIG};
use crate::constants;
use crate::xml::Xml;

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
    pub async fn get_tree_title(&self, _db: &Db) -> String {
        log::trace!("In get_tree_title");

        assert!(self.parent.borrow().is_none(), "Expected to receive root node");

        // Assuming, for now, it is good enough to use text node under title tag
        // which can be done classically

        let children = self.children.borrow();
        let html = children.first().unwrap();
        let children = html.children.borrow();
        let head = children.iter().find(|item| {
            item.xml.get_element_tag_name() == "head"
        }).unwrap();
        let children = head.children.borrow();
        let title = children.iter().find(|item| {
            item.xml.get_element_tag_name() == "title"
        }).unwrap();
        let children = title.children.borrow();
        let text = children.first().unwrap().xml.to_string();
        
        let text = String::from(text.trim_matches(|c| c == ' ' || c == '\n'));

        text
    }

    pub async fn interpret_node(&self, db: &Db, output_tree: &Rc<Node>) -> (Vec<NodeData>, bool) {
        log::trace!("In interpret_node");

        if let Some(classical_interpretation) = self.interpret_node_classically() {
            log::info!("Node interpreted classically");
            return (classical_interpretation, false);
        }

        let key = &self.xml.to_hash();
        log::debug!("key: {}", key);

        let cache = get_node_data(&db, &key)
            .expect("Could not get node data from database");

        if let Some(cache) = cache {
            log::info!("Cache hit!");
            return (cache.clone(), false);
        } else {
            log::info!("Cache miss!");

            let surrounding_xml: String = self.node_to_xml_snippet_with_context();

            let examples: Vec<Xml> = self.get_examples(output_tree);

            log::debug!("example.len(): {}", examples.len());

            for example in examples.iter() {
                log::debug!("example: {}", example.to_string());
            }

            let llm_result: Vec<NodeData> = llm::xml_to_data(&self.xml, surrounding_xml, examples)
                .await
                .expect("LLM unable to generate node data");

            store_node_data(&db, &key, llm_result.clone())
                .expect("Unable to persist node data to database");

            return (llm_result.clone(), true);
        }
    }

    fn get_examples(&self, output_tree: &Rc<Node>) -> Vec<Xml> {
         let mut lineage = self.get_lineage();

         lineage.pop_front(); // root node

         let mut examples = find_all_node_xml_by_lineage(
             output_tree.clone(),
             lineage.clone(),
         );

         // assuming first example is already present in basis tree
         if !examples.is_empty() {
             examples.remove(0);
         }

         let config = CONFIG.lock().unwrap();
         let max_examples = config.llm.target_node_examples_max_count;
         let number_to_take = std::cmp::min(max_examples, examples.len());
         
         examples[..number_to_take].to_vec()
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
                take_from_end(&document.0),
                document.2,
                take_from_start(&document.4),
            )
        } else {
            let after_start_tag = &format!(
                "{}{}{}",
                document.2,
                document.3,
                document.4
            );

            format!(
                "{}<!--Target node start -->{}<!--Target node end -->{}",
                take_from_end(&document.0),
                document.1,
                take_from_start(after_start_tag),
            )
        }
    }

    fn interpret_node_classically(&self) -> Option<Vec<NodeData>> {
        log::trace!("In interpret_node_classically");

        let attributes = self.xml.get_attributes();

        // * Root node
        if self.hash == constants::ROOT_NODE_HASH {
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

        // * script
        if self.xml.get_element_tag_name() == "script" {
            log::info!("Node represents script HTML element. We ignore these, for now...");
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

fn take_from_end(s: &str) -> &str {
     let config = CONFIG.lock().unwrap();
     let len = s.len();
     if config.llm.target_node_adjacent_xml_length >= len {
         s
     } else {
         let start_index = len - config.llm.target_node_adjacent_xml_length;
         let mut adjusted_start = start_index;

         while !s.is_char_boundary(adjusted_start) && adjusted_start < len {
             adjusted_start += 1;
         }

         &s[adjusted_start..]
     }
 }

 fn take_from_start(s: &str) -> &str {
     let config = CONFIG.lock().unwrap();
     if config.llm.target_node_adjacent_xml_length >= s.len() {
         s
     } else {
         let end_index = config.llm.target_node_adjacent_xml_length;
         let mut adjusted_end = end_index;

         while !s.is_char_boundary(adjusted_end) && adjusted_end > 0 {
adjusted_end -= 1;
         }

         &s[..adjusted_end]
     }
 }

