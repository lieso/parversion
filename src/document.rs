use std::collections::HashSet;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use uuid::Uuid;

use crate::models::*;
use crate::tree;

pub fn harvest_json(node: &Node) -> Document {

    let mut document = Document::new();

    tree::post_order_traversal(node, &mut |n| document.visit_node(n));

    document
}

fn get_complex_type_id(set: HashSet<String>) -> String {
    let mut vec: Vec<&String> = set.iter().collect();
    vec.sort();

    let mut hasher = Sha256::new();

    for item in vec {
        hasher.update(item.as_bytes());
    }

    let hash = hasher.finalize();

    format!("{:x}", hash)
}

impl Document {

    fn new() -> Self {
        Document {
            node_complex_object: HashMap::new(),
            complex_types: Vec::new(),
            complex_objects: HashMap::new(),
        }
    }

    fn visit_node(&mut self, node: &Node) {
        if node.children.is_empty() {
            return;
        }

        let mut set = node.to_hash_set();

        for child in &node.children {

            if let Some(complex_object) = self.node_complex_object.get(&node.hash) {
                set.extend(complex_object.set.clone());
            } else {
                set.extend(child.to_hash_set());
            }
        }


        let keys: HashSet<String> = set
            .iter()
            .cloned()
            .map(|(first, _)| first)
            .collect();

        let complex_type_id: String = get_complex_type_id(keys.clone());

        if let Some(_complex_type) = self.get_complex_type_by_id(&complex_type_id) {
            let complex_object = ComplexObject::new(complex_type_id.clone(), set.clone());

            self.complex_objects
                .entry(complex_type_id)
                .or_insert_with(Vec::new)
                .push(complex_object);
        } else {
            let complex_type = ComplexType::new(complex_type_id.clone(), keys.clone());

            self.complex_types.push(complex_type);

            let complex_object = ComplexObject::new(complex_type_id.clone(), set.clone());

            self.complex_objects
                .entry(complex_type_id)
                .or_insert_with(Vec::new)
                .push(complex_object);
        }
    }

    fn get_complex_type_by_id(&self, id: &str) -> Option<&ComplexType> {
        self.complex_types.iter().find(|&item| item.id == id)
    }
}


impl ComplexType {
    fn new(id: String, set: HashSet<String>) -> Self {
        ComplexType {
            id: id,
            set: set,
        }
    }
}

impl ComplexObject {
    fn new(type_id: String, set: HashSet<(String, String)>) -> Self {
        ComplexObject {
            id: Uuid::new_v4().to_string(),
            type_id: type_id,
            set: set,
        }
    }
}
