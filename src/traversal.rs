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
    pub metadata: Option<TreeMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadata {
    pub is_id: bool,
    pub is_url: bool,
    pub is_page_link: bool,
    pub is_action_link: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValue {
    pub meta: ContentMetadata,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub values: Vec<ContentValue>,
    pub children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub data: Content,
    pub metadata: Option<TreeMetadata>,
}

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    //XML,
    //CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

impl Content {
    pub fn remove_empty(&mut self) {
        self.children.iter_mut().for_each(|child| child.remove_empty());
        self.children.retain(|child| !child.is_empty());
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty() && self.children.is_empty()
    }
}

fn process_node(
    content_node: &Rc<Node>,
    basis_tree: &Rc<Node>,
    content: &mut Content
) {
    let lineage = content_node.get_lineage();
    let basis_node = search_tree_by_lineage(Rc::clone(basis_tree), lineage.clone()).unwrap();

    for node_data in basis_node.data.borrow().iter() {
        if content_node.xml.is_text() && !node_data.text_fields.clone().unwrap().is_informational {
            log::info!("Ignoring non-informational text node");
            continue;
        }

        let content_value = ContentValue {
            name: node_data.name.clone(),
            value: node_data.value(&content_node.xml),
            meta: ContentMetadata {
                is_id: node_data.element_fields.clone().map_or(false, |fields| fields.is_id),
                is_url: node_data.element_fields.clone().map_or(false, |fields| fields.is_url),
                is_page_link: node_data.element_fields.clone().map_or(false, |fields| fields.is_page_link),
                is_action_link: node_data.element_fields.clone().map_or(false, |fields| fields.is_action_link),
            },
        };

        content.values.push(content_value);
    }
}

impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            output_tree: tree,
            basis_tree: None,
            metadata: None,
        }
    }

    pub fn with_basis(mut self, tree: Rc<Node>) -> Self {
        self.basis_tree = Some(Rc::clone(&tree));
        
        self
    }

    pub fn with_metadata(mut self, metadata: TreeMetadata) -> Self {
        self.metadata = Some(metadata);

        self
    }

    pub fn harvest(mut self) -> Result<String, Errors> {
        let mut content = Content {
            values: Vec::new(),
            children: Vec::new(),
        };

        fn recurse(
            mut content_node: Rc<Node>,
            basis_tree: Rc<Node>,
            content: &mut Content,
        ) {
            if content_node.is_linear_tail() {
                panic!("Did not expect to encounter node in linear tail");
            }

            if content_node.is_linear_head() {
                log::info!("Content node is head of linear sequence of nodes");

                while content_node.is_linear() {
                    process_node(&content_node, &basis_tree, content);

                    content_node = {
                        let next_node = content_node.children.borrow().first().expect("Linear content node has no children").clone();
                        next_node.clone()
                    };
                }

                process_node(&content_node, &basis_tree, content);
            } else {
                log::info!("Content node is non-linear");

                process_node(&content_node, &basis_tree, content);
            }

            for child in content_node.children.borrow().iter() {
                let mut child_content = Content {
                    values: Vec::new(),
                    children: Vec::new(),
                };

                recurse(
                    child.clone(),
                    basis_tree.clone(),
                    &mut child_content,
                );

                content.children.push(child_content);
            }
        }

        recurse(
            self.output_tree.clone(),
            self.basis_tree.clone().unwrap(),
            &mut content,
        );

        log::info!("Removing empty objects from content...");
        content.remove_empty();

        let output = Output {
           data: content,
           metadata: self.metadata,
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
