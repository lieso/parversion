use serde::{Serialize, Deserialize};
use std::rc::{Rc};
use std::collections::{HashMap, VecDeque};

use crate::node::*;
use crate::error::{Errors};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexType {
    pub id: String,
    pub values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DerivedType {
    pub id: String,
    pub complex_mapping: HashMap<String, HashMap<String, String>>,
    pub values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexObject {
    pub id: String,
    pub type_id: String,
    pub values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Relationship {
    pub id: String,
    pub complex_type_id: String,
    pub origin_field: String,
    pub target_field: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Rc<Node>,
    pub basis_tree: Option<Rc<Node>>,
    pub primitives: Vec<HashMap<String, String>>,
    pub complex_types: Vec<ComplexType>,
    pub complex_objects: Vec<ComplexObject>,
    pub lists: Vec<String>,
    pub relationships: Vec<Relationship>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub complex_types: HashMap<String, Vec<ComplexType>>,
    pub complex_objects: Vec<ComplexObject>,
    pub lists: HashMap<String, Vec<String>>,
    pub relationships: HashMap<String, Relationship>,
}

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    //XML,
    //CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

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

            let lineage = current.get_lineage();
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
