use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use std::process::{Command, Stdio};
use std::io::Write;
use regex::Regex;

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData, bft};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::error::{Errors};
use crate::macros::*;
use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Graph<XmlNode>,
    pub basis_graph: Option<Graph<BasisNode>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValueMetadata {
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValue {
    pub meta: ContentValueMetadata,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadataRecursive {
    pub is_root: bool,
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadata {
    pub recursive: Option<ContentMetadataRecursive>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub id: String,
    pub meta: ContentMetadata,
    pub values: Vec<ContentValue>,
    pub children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub data: Content,
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







fn sanitize_awk_expression(input: &str) -> Option<String> {
    let re = Regex::new(r"^awk\s*'([^']*)'$").expect("Failed to create regex");

    re.captures(input).and_then(|caps| {
        caps.get(1).map(|matched_text| matched_text.as_str().to_string())
    })
}







fn process_node(
    output_node: Graph<XmlNode>,
    basis_graph: Graph<BasisNode>,
    content: &mut Content,
) {
    let output_node_xml: XmlNode = read_lock!(output_node).data.clone();
    let lineage = get_lineage(Arc::clone(&output_node));
    let basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), lineage);

    let data = read_lock!(basis_node).data.data.clone();
    for node_data in read_lock!(data).iter() {
        if let Some(text_data) = &node_data.text {
            if text_data.is_presentational {
                log::info!("Discarding presentational text node data");
                continue;
            }
        }

        if let Some(element_data) = &node_data.element {
            if element_data.attribute == "href" && !element_data.is_page_link {
                log::info!("Discarding href action link...");
                continue;
            }
        }

        let content_value = ContentValue {
            name: node_data.name.clone(),
            value: node_data.value(&output_node_xml),
            meta: ContentValueMetadata {
                is_primary_content: node_data.clone().text.map_or(false, |text| text.is_primary_content)
            },
        };

        content.values.push(content_value);
    }

    let structures = read_lock!(basis_node).data.structure.clone();
    for structure in read_lock!(structures).iter() {












        if let Some(recursive_attribute) = &structure.recursive_attribute {

            if recursive_attribute.starts_with('@') {
                let attribute = &recursive_attribute[1..];

                bft(Arc::clone(&output_node), &mut |node: Graph<XmlNode>| {

                    let xml = read_lock!(node).data.clone();

                    if let Some(xml_value) = xml.get_attribute_value(attribute) {
                        let root_node_attribute_values = &structure.root_node_attribute_values.clone().unwrap();

                        if root_node_attribute_values.contains(&xml_value) {
                            log::info!("Detected root node");

                            let meta = ContentMetadataRecursive {
                                is_root: true,
                                parent_id: None,
                            };
                            
                            content.meta.recursive = Some(meta);
                        } else {
                            log::info!("Detected recursive non-root node");
                            let parent_node_attribute_value = &structure.parent_node_attribute_value.clone().unwrap();

                            log::debug!("parent_node_attribute_value: {}", parent_node_attribute_value);
                            
                            if let Some(awk_expression) = sanitize_awk_expression(&parent_node_attribute_value) {
                                log::debug!("awk_expression: {}", awk_expression);

                                let mut process = Command::new("awk")
                                    .arg(awk_expression)
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn()
                                    .expect("Failed to spawn awk process");

                                let input_data = format!("\"{}\"", xml_value);

                                if let Some(mut stdin) = process.stdin.take() {
                                    stdin.write_all(input_data.as_bytes()).expect("Failed to write to stdin");
                                }

                                let output = process
                                    .wait_with_output()
                                    .expect("Failed to read awk output");

                                if output.status.success() {
                                    let parent_node_recursive_attribute_value = String::from_utf8_lossy(&output.stdout);
                                    log::debug!("parent_node_recursive_attribute_value: {}", parent_node_recursive_attribute_value);




                                    let rl = read_lock!(output_node);
                                    let output_node_parent = rl.parents.get(0).unwrap();


                                    let mut siblings: Vec<Graph<XmlNode>> = Vec::new();
                                    for sibling in read_lock!(output_node_parent).children.iter() {
                                        if read_lock!(sibling).id == read_lock!(output_node).id {
                                            break;
                                        }

                                        siblings.push(Arc::clone(&sibling));
                                    }

                                    siblings.reverse();




                                    let mut found_target = false;
                                    for sibling in siblings.iter() {

                                        if found_target {
                                            break;
                                        }

                                        log::debug!("sibling: {}", read_lock!(sibling).data.describe());


                                        bft(Arc::clone(&sibling), &mut |inner_node: Graph<XmlNode>| {

                                            let inner_xml = read_lock!(inner_node).data.clone();

                                            if !inner_xml.is_element() {
                                                return true;
                                            }

                                            if let Some(inner_xml_value) = inner_xml.get_attribute_value(attribute) {
                                                if inner_xml_value.to_string().trim() == parent_node_recursive_attribute_value.to_string().trim() {


                                                    let meta = ContentMetadataRecursive {
                                                        is_root: false,
                                                        parent_id: Some(read_lock!(sibling).id.clone()),
                                                    };

                                                    content.meta.recursive = Some(meta);

                                                    
                                                    found_target = true;

                                                    return false;
                                                }
                                            }

                                            true
                                        });

                                    }

                                    








                                } else {

                                }

                            }


                        }


                        return false;
                    }

                    true
                });

            }


        }

















    }
}

impl Traversal {
    pub fn from_tree(output_tree: Graph<XmlNode>) -> Self {
        Traversal {
            output_tree: output_tree,
            basis_graph: None,
        }
    }

    pub fn with_basis(mut self, graph: Graph<BasisNode>) -> Self {
        self.basis_graph = Some(Arc::clone(&graph));

        self
    }

    pub fn harvest(self) -> Result<String, Errors> {
        let mut content = Content {
            id: read_lock!(self.output_tree).id.clone(),
            meta: ContentMetadata {
                recursive: None,
            },
            values: Vec::new(),
            children: Vec::new(),
        };

        fn recurse(
            mut output_node: Graph<XmlNode>,
            basis_graph: Graph<BasisNode>,
            output_content: &mut Content,
        ) {
            if read_lock!(output_node).is_linear_tail() {
                panic!("Did not expect to encounter node in linear tail");
            }

            if read_lock!(output_node).is_linear_head() {
                log::info!("Output node is head of linear sequence of nodes");

                while read_lock!(output_node).is_linear() {
                    process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);

                    output_node = {
                        let next_node = read_lock!(output_node).children.first().expect("Linear output node has no children").clone();
                        next_node.clone()
                    };
                }

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);
            } else {
                log::info!("Output node is non-linear");

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);
            }

            for child in read_lock!(output_node).children.iter() {
                let mut child_content = Content {
                    id: read_lock!(child).id.clone(),
                    meta: ContentMetadata {
                        recursive: None,
                    },
                    values: Vec::new(),
                    children: Vec::new(),
                };

                recurse(
                    Arc::clone(child),
                    Arc::clone(&basis_graph),
                    &mut child_content,
                );

                output_content.children.push(child_content);
            }
        }

        recurse(
            Arc::clone(&self.output_tree),
            Arc::clone(&self.basis_graph.unwrap()),
            &mut content,
        );

        log::info!("Removing empty objects from content...");
        content.remove_empty();

        let output = Output {
            data: content,
        };

        let output_format = DEFAULT_OUTPUT_FORMAT;
        log::debug!("output_format: {:?}", output_format);

        match output_format {
            OutputFormats::JSON => {
                log::info!("Harvesting tree as JSON");

                let serialized = serde_json::to_string(&output).expect("Could not serialize output to JSON");

                Ok(serialized)
            },
        }
    }
}
