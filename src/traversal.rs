use serde::{Serialize, Deserialize};
use std::rc::{Rc};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::node::*;
use crate::error::{Errors};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexType {
    pub id: String,
    pub name: String,
    pub fields: Vec<String>,
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
    pub complex_objects: Vec<String>,
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
    pub object_count: u64,
    pub type_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutputMeta {
    object_count: u64,
    type_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub complex_types: HashMap<String, ComplexType>,
    pub complex_objects: HashMap<String, ComplexObject>,
    pub lists: HashMap<String, Vec<String>>,
    pub relationships: HashMap<String, Relationship>,
    pub meta: OutputMeta,
}

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    //XML,
    //CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

pub fn map_complex_object(basis_tree: Rc<Node>, output_tree: Rc<Node>, complex_type: &ComplexType) -> ComplexObject {
    log::trace!("In map_complex_object");

    let mut values: HashMap<String, String> = HashMap::new();
    let mut complex_objects: Vec<String> = Vec::new();

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
            complex_objects.push(child.id.clone());
        } else {
            values.extend(
                node_data_to_hash_map(&basis_child.data, Rc::clone(&output_tree)).drain()
            );
        };
    }

    ComplexObject {
        id: output_tree.id.clone(),
        type_id: complex_type.id.to_string(),
        values: values,
        complex_objects: complex_objects,
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
            object_count: 0,
            type_count: 0,
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

            if let Some(basis_node) = search_tree_by_lineage(Rc::clone(&basis_tree), lineage.clone()) {

                if let Some(complex_type_name) = basis_node.complex_type_name.borrow().as_ref() {

                    let complex_type = self.complex_types.iter().find(|item| item.name == *complex_type_name);

                    if let Some(complex_type) = complex_type {
                        let complex_object = map_complex_object(basis_node.clone(), current.clone(), complex_type);
                        self.complex_objects.push(complex_object);
                    } else {
                        let complex_type = ComplexType {
                            id: Uuid::new_v4().to_string(),
                            name: complex_type_name.to_string(),
                            fields: basis_node.data.borrow().iter().map(|item| {
                                item.name.to_string()
                            }).collect()
                        };
                        self.type_count += 1;
                        self.complex_types.push(complex_type.clone());

                        let complex_object = map_complex_object(basis_node.clone(), current.clone(), &complex_type);
                        self.complex_objects.push(complex_object);
                    };

                    self.object_count += 1;
                };

            } else {
                log::warn!("Basis tree does to contain corresponding node to output tree!");
                continue;
            }

            for child in current.children.borrow().iter() {
                bfs.push_back(child.clone());
            }
        }

        Ok(self)
    }

    pub fn harvest(self) -> Result<String, Errors> {
        let mut output = Output {
            complex_types: HashMap::new(),
            complex_objects: HashMap::new(),
            lists: HashMap::new(),
            relationships: HashMap::new(),
            meta: OutputMeta {
                object_count: self.object_count,
                type_count: self.type_count,
            }
        };

        for complex_type in self.complex_types.iter() {
            output.complex_types.insert(
                complex_type.id.to_string(),
                complex_type.clone()
            );
        }

        for complex_object in self.complex_objects.iter() {
            output.complex_objects.insert(
                complex_object.id.to_string(),
                complex_object.clone()
            );
        }

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
