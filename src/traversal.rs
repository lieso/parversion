use serde::{Serialize, Deserialize};
use std::rc::{Rc};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use std::fs::{OpenOptions};
use std::io::{Write};

use crate::node::*;
use crate::error::{Errors};

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

impl Output {
    pub fn remove_empty(&mut self) {
        self.children.iter_mut().for_each(|child| child.remove_empty());
        self.children.retain(|child| !child.is_empty());
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty() && self.children.is_empty()
    }
}

impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            output_tree: tree,
            basis_tree: None,
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

        log::info!("Removing empty objects from output...");
        output.remove_empty();

        //=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>=>
        //
        // Making an assumption here that root output represents <html> element
        // and that it can be ignored. 
        // We do this for slightly cleaner JSON output
        //
        //<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=<=
        if let Some(first_child) = output.children.get_mut(0) {
            output.values = std::mem::take(&mut first_child.values);
            output.children = std::mem::take(&mut first_child.children);
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
