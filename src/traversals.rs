use std::collections::{HashMap, VecDeque};
use std::rc::{Rc};
use std::cell::RefCell;

use crate::models::*;
use crate::utilities;

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    XML,
    CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

pub fn map_primitives(basis_tree: Rc<Node>, output_tree: Rc<Node>) -> HashMap<String, String> {
    unimplemented!()
}

pub fn node_data_to_hash_map(node_data: &RefCell<Vec<NodeData>>, output_tree: Rc<Node>) -> HashMap<String, String> {
    log::trace!("In node_data_to_hash_map");

    let mut values: HashMap<String, String> = HashMap::new();

    for item in node_data.borrow().iter() {
        if let Ok(output_tree_value) = utilities::apply_xpath(&output_tree.xml, &item.xpath) {
            values.insert(item.name.clone(), output_tree_value.clone());
        } else {
            log::warn!("xpath from basis tree could not be applied to output tree");
        }
    }

    values
}

pub fn map_complex_object(basis_tree: Rc<Node>, output_tree: Rc<Node>) -> ComplexObject {
    log::trace!("In map_complex_object");

    let maybe_complex_type_name = basis_tree.complex_type_name.borrow();
    let type_id_placeholder = maybe_complex_type_name.as_ref().unwrap();

    let mut values: HashMap<String, String> = HashMap::new();

    values.extend(
        node_data_to_hash_map(&basis_tree.data, Rc::clone(&output_tree)).drain()
    );

    for child in output_tree.children.borrow().iter() {
        let basis_children_ref = basis_tree.children.borrow();
        let basis_child = basis_children_ref
            .iter()
            .find(|item| item.hash == child.hash)
            .unwrap();

        if let Some(complex_type_name) = basis_child.complex_type_name.borrow().as_ref() {
            values.insert(child.id.clone(), complex_type_name.clone());
        } else {
            values.extend(
                node_data_to_hash_map(&basis_child.data, Rc::clone(&output_tree)).drain()
            );
        };
    }

    ComplexObject {
        id: output_tree.id.clone(),
        type_id: type_id_placeholder.to_string(),
        values: values,
    }
}

pub fn search_tree_by_lineage(mut tree: Rc<Node>, mut lineage: VecDeque<String>) -> Option<Rc<Node>> {
    log::trace!("In search_tree_by_lineage");

    //if let Some(hash) = lineage.pop_front() {
    //    if tree.hash != hash {
    //        return None;
    //    }
    //} else {
    //    return None;
    //}

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
            return None;
        }
    }

    Some(tree)
}

pub fn get_lineage(node: Rc<Node>) -> VecDeque<String> {
    let mut lineage = VecDeque::new();
    lineage.push_back(node.hash.clone());

    let mut current_parent = node.parent.borrow().clone();

    while let Some(parent) = current_parent {
        lineage.push_front(parent.hash.clone());

        current_parent = {
            let node_ref = parent.parent.borrow();
            node_ref.as_ref().map(|node| node.clone())
        };
    }

    lineage
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

impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            output_tree: tree,
            basis_tree: None,
            primitives: Vec::new(),
            complex_types: Vec::new(),
            complex_objects: Vec::new(),
            lists: Vec::new(),
            relationships: Vec::new(),
        }
    }

    pub fn with_basis(mut self, tree: Rc<Node>) -> Self {
        self.basis_tree = Some(Rc::clone(&tree));
        
        self
    }

    pub fn traverse(mut self) -> Result<Self, Errors> {
        let basis_tree = self.basis_tree.clone().unwrap();

        let mut bfs: VecDeque<Rc<Node>> = VecDeque::new();
        bfs.push_back(Rc::clone(&self.output_tree));

        let mut node_count = 1;

        while let Some(current) = bfs.pop_front() {
            log::info!("Traversing node #{}", node_count);
            node_count = node_count + 1;

            let lineage = get_lineage(Rc::clone(&current));
            log::debug!("lineage: {:?}", lineage);

            if let Some(basis_node) = search_tree_by_lineage(basis_tree.clone(), lineage.clone()) {

                if basis_node.complex_type_name.borrow().is_some() {
                    let complex_object = map_complex_object(basis_node, current.clone());
                    log::debug!("complex_object: {:?}", complex_object);

                    self.complex_objects.push(complex_object);
                }

            } else {
                log::warn!("Basis tree does to contain corresponding node to output tree!");
                //continue;
            }

            for child in current.children.borrow().iter() {
                bfs.push_back(child.clone());
            }
        }

        Ok(self)
    }

    pub fn harvest(self) -> Result<String, Errors> {
        let output = Output {
            complex_types: HashMap::new(),
            complex_objects: self.complex_objects.clone(),
            lists: HashMap::new(),
            relationships: HashMap::new(),
        };

        let output_format = DEFAULT_OUTPUT_FORMAT;
        log::debug!("output_format: {:?}", output_format);

        match output_format {
            OutputFormats::JSON => {
                log::info!("Harvesting tree as JSON");

                let serialized = serde_json::to_string(&output).expect("Could not serialize output to JSON");

                Ok(serialized)
            },
            _ => {
                log::error!("Unexpected output format: {:?}", output_format);
                Err(Errors::UnexpectedOutputFormat)
            }
        }
    }
}
