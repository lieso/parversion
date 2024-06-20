use serde::{Serialize, Deserialize};
use std::rc::{Rc};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use std::fs::{OpenOptions};
use std::io::{Write};

use crate::node::*;
use crate::error::{Errors};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexType {
    pub id: String,
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexObject {
    pub id: String,
    pub parent_id: Option<String>,
    pub type_id: String,
    pub values: HashMap<String, HashMap<String, String>>,
    pub depth: u16,
    pub complex_objects: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DerivedType {
    pub id: String,
    pub complex_mapping: HashMap<String, HashMap<String, String>>,
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
pub struct ColorPalette {
    pub one: String,
    pub two: String,
    pub three: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Rc<Node>,
    pub basis_tree: Option<Rc<Node>>,
    pub primitives: Vec<HashMap<String, String>>,
    pub complex_types: Vec<ComplexType>,
    pub complex_objects: Vec<ComplexObject>,
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
    pub values: HashMap<String, String>,
    pub children: Vec<Output>,
}

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    //XML,
    //CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

fn map_linear_nodes(basis_node: Rc<Node>, output_node: Rc<Node>) -> ComplexObject {
    unimplemented!()
}

fn map_nonlinear_nodes(basis_node: Rc<Node>, output_node: Rc<Node>) -> ComplexObject {
    unimplemented!()
}








impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            output_tree: tree,
            basis_tree: None,
            primitives: Vec::new(),
            complex_types: Vec::new(),
            complex_objects: Vec::new(),
            relationships: Vec::new(),
            object_count: 0,
            type_count: 0,
        }
    }

    pub fn with_basis(mut self, tree: Rc<Node>) -> Self {
        self.basis_tree = Some(Rc::clone(&tree));
        
        self
    }

    pub fn harvest(mut self) -> Result<String, Errors> {
        let mut output = Output {
            values: HashMap::new(),
            children: Vec::new(),
        };

        fn recurse(
            mut output_node: Rc<Node>,
            basis_tree: Rc<Node>,
            output: &mut Output,
        ) {
            if output_node.is_linear_tail() {
                panic!("Did not expect to encounter node in linear tail");
            }

            if output_node.is_linear_head() {
                log::info!("Output node is head of linear sequence of nodes");

                while output_node.is_linear() {
                    let lineage = output_node.get_lineage();
                    let basis_node = search_tree_by_lineage(Rc::clone(&basis_tree), lineage.clone()).unwrap();

                    for node_data in basis_node.data.borrow().iter() {
                        let output_value = node_data.value(&output_node.xml);
                        output.values.insert(node_data.name.clone(), output_value.clone());
                    }

                    output_node = {
                        let next_node = output_node.children.borrow().first().expect("Linear output node has no children").clone();
                        next_node.clone()
                    };
                }

            } else {
                log::info!("Output node is non-linear");

                let lineage = output_node.get_lineage();
                let basis_node = search_tree_by_lineage(Rc::clone(&basis_tree), lineage.clone()).unwrap();

                for node_data in basis_node.data.borrow().iter() {
                    let output_value = node_data.value(&output_node.xml);
                    output.values.insert(node_data.name.clone(), output_value.clone());
                }
            }

            for child in output_node.children.borrow().iter() {
                let mut child_output = Output {
                    values: HashMap::new(),
                    children: Vec::new(),
                };

                recurse(
                    child.clone(),
                    basis_tree.clone(),
                    &mut child_output,
                );

                output.children.push(child_output);
            }
        }

        recurse(
            self.output_tree.clone(),
            self.basis_tree.clone().unwrap(),
            &mut output,
        );

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
