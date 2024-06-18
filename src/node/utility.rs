use std::collections::{VecDeque};
use sha2::{Sha256, Digest};

use super::Node;

pub fn generate_element_node_hash(tags: Vec<String>, attributes: Vec<String>) -> String {
    let mut hasher = Sha256::new();
    
    let mut hasher_items = Vec::new();

    for tag in tags.iter() {
        hasher_items.push(tag.to_string());
    }

    for attribute in attributes.iter() {
        hasher_items.push(attribute.to_string());
    }

    hasher_items.sort();

    hasher.update(hasher_items.join(""));

    format!("{:x}", hasher.finalize())
}

impl Node {
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

    pub fn get_lineage(&self) -> VecDeque<String> {
        let mut lineage = VecDeque::new();
        lineage.push_back(self.hash.clone());

        let mut current_parent = self.parent.borrow().clone();

        while let Some(parent) = current_parent {
            lineage.push_front(parent.hash.clone());

            current_parent = {
                let node_ref = parent.parent.borrow();
                node_ref.as_ref().map(|node| node.clone())
            };
        }

        lineage
    }

    pub fn get_depth(&self) -> u16 {
        let mut depth = 0;

        let mut current_parent = self.parent.borrow().clone();

        while let Some(parent) = current_parent {
            depth += 1;

            current_parent = {
                let node_ref = parent.parent.borrow();
                node_ref.as_ref().map(|node| node.clone())
            };
        }

        depth
    }

    pub fn is_linear(&self) -> bool {
        let is_linear = self.children.borrow().len() == 1 && self.parent.borrow().is_some();
        log::debug!("Node is {}", if is_linear { "linear" } else { "not linear" });
        is_linear
    }

    pub fn is_linear_head(&self) -> bool {
        if self.is_linear() {
            if self
                .parent
                .borrow()
                .as_ref()
                .map_or(false, |parent| !parent.is_linear())
            {
                return true;
            }
        }

        false
    }

    pub fn is_linear_tail(&self) -> bool {
        self.is_linear() && !self.is_linear_head()
    }

    pub fn is_structural(&self) -> bool {
        self.xml.get_attributes().is_empty()
    }
}
