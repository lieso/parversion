use serde::{Serialize, Deserialize};
use std::sync::{Arc};
use std::process::{Command, Stdio};
use regex::Regex;
use std::io::Write;

use parversion::graph_node::{Graph, bft, get_lineage, apply_lineage};
use parversion::xml_node::{XmlNode};
use parversion::basis_node::{BasisNode};
use parversion::macros::*;
use parversion::content::{
    ContentMetadataRecursive,
    ContentMetadataEnumerative,
    ContentMetadata
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecursiveStructure {
    pub recursive_attribute: Option<String>,
    pub root_node_attribute_values: Option<Vec<String>>,
    pub parent_node_attribute_value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnumerativeStructure {
    pub intrinsic_component_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssociativeStructure {
    pub subgraph_ids: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataStructure {
    pub recursive: Option<RecursiveStructure>,
    pub enumerative: Option<EnumerativeStructure>,
    pub associative: Option<AssociativeStructure>,
}

pub fn apply_structure(
    structure: NodeDataStructure,
    output_node: Graph<XmlNode>,
    basis_graph: Graph<BasisNode>,
) -> ContentMetadata {
    log::trace!("In apply_structure");

    let mut meta = ContentMetadata {
        recursive: None,
        enumerative: None,
        associative: None,
    };

    if let Some(recursive_structure) = structure.recursive {
        if let (
            Some(recursive_attribute),
            Some(root_node_attribute_values),
            Some(parent_node_attribute_value)
        ) = (
            recursive_structure.recursive_attribute,
            recursive_structure.root_node_attribute_values,
            recursive_structure.parent_node_attribute_value
        ) {
            meta.recursive = apply_recursive_structure(
                recursive_attribute,
                root_node_attribute_values,
                parent_node_attribute_value,
                Arc::clone(&output_node),
            );
        }
    }

    if let Some(enumerative_structure) = structure.enumerative {
        let intrinsic_component_ids = enumerative_structure.intrinsic_component_ids;

        let rl = read_lock!(output_node);
        let output_node_parent = rl.parents.first().cloned().unwrap();
        let output_node_parent_children = &read_lock!(output_node_parent).children;

        let mut found_current_node = false;

        let next_node = output_node_parent_children.iter().find(|node| {
            if found_current_node {
                let lineage = get_lineage(Arc::clone(&node));
                if let Some(basis_node) = apply_lineage(Arc::clone(&basis_graph), lineage) {
                    return &read_lock!(basis_node).id == intrinsic_component_ids.first().unwrap();
                }
            } else {
                if rl.id == read_lock!(node).id {
                    found_current_node = true;
                }
            }

            false
        });

        let next_id: Option<String> = if let Some(next_node) = next_node {
            Some(read_lock!(next_node).id.clone())
        } else {
            None
        };

        let content_metadata_enumerative = ContentMetadataEnumerative {
            next_id,
        };

        meta.enumerative = Some(content_metadata_enumerative);
    }

    meta
}

fn apply_recursive_structure(
    recursive_attribute: String,
    root_node_attribute_values: Vec<String>,
    parent_node_attribute_value: String,
    output_node: Graph<XmlNode>,
) -> Option<ContentMetadataRecursive> {
    log::trace!("In apply_recursive_structure");
    log::debug!("recursive_attribute: {}", recursive_attribute);
    log::debug!("root_node_attribute_values: {:?}", root_node_attribute_values);
    log::debug!("parent_node_attribute_value: {}", parent_node_attribute_value);

    let mut meta: Option<ContentMetadataRecursive> = None;

    if recursive_attribute.starts_with('@') {
        let attribute = &recursive_attribute[1..];

        bft(Arc::clone(&output_node), &mut |node: Graph<XmlNode>| {
            let xml = read_lock!(node).data.clone();

            if let Some(xml_value) = xml.get_attribute_value(attribute) {
                if root_node_attribute_values.contains(&xml_value) {
                    log::info!("Detected root node");

                    meta = Some(ContentMetadataRecursive {
                        is_root: true,
                        parent_id: None,
                    });

                    return false;
                } else {
                    log::info!("Detected recursive child node");

                    if let Some(parent_node_recursive_attribute_value) = evaluate_awk_expression(
                        parent_node_attribute_value.clone(),
                        xml_value.clone()
                    ) {
                        if let Some(parent_node) = find_parent_node(
                            Arc::clone(&output_node),
                            attribute.to_string(),
                            parent_node_recursive_attribute_value.clone(),
                        ) {
                            log::info!("Found parent node by recursive attribute value");
                            
                            meta = Some(ContentMetadataRecursive {
                                is_root: false,
                                parent_id: Some(read_lock!(parent_node).id.clone()),
                            });

                            return false;
                        } else {
                            log::warn!("Could not find parent node with attribute value: {}", parent_node_recursive_attribute_value);
                        }
                    } else {
                        log::warn!("Could not evaluate awk expression");
                    }
                }
            }

            true
        });
    } else {
        log::warn!("Unexpected recursive attribute: {}", recursive_attribute);
    }

    meta
}

fn find_parent_node(output_node: Graph<XmlNode>, attribute: String, attribute_value: String) -> Option<Graph<XmlNode>> {
    log::trace!("In find_parent_node");

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
    
    let mut parent_node: Option<Graph<XmlNode>> = None;

    let mut found_target = false;
    for sibling in siblings.iter() {
        if found_target {
            break;
        }

        bft(Arc::clone(&sibling), &mut |inner_node: Graph<XmlNode>| {
            let inner_xml = read_lock!(inner_node).data.clone();

            if !inner_xml.is_element() {
                return true;
            }

            if let Some(inner_xml_value) = inner_xml.get_attribute_value(&attribute) {
                if inner_xml_value.to_string().trim() == attribute_value.to_string().trim() {
                    parent_node = Some(Arc::clone(&sibling));
                    found_target = true;

                    return false;
                }
            }

            true
        });
    }

    parent_node
}

fn evaluate_awk_expression(expression: String, input_data: String) -> Option<String> {
    log::trace!("In evaluate_awk_expression");

    let input_data = format!("{}", input_data);

    if let Some(awk_expression) = sanitize_awk_expression(&expression) {
        log::debug!("awk_expression: {}", awk_expression);
        log::debug!("input_data: {}", input_data);

        let mut process = Command::new("awk")
            .arg(awk_expression)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn awk process");

        if let Some(mut stdin) = process.stdin.take() {
            stdin.write_all(input_data.as_bytes()).expect("Failed to write to stdin");
        }

        let output = process
            .wait_with_output()
            .expect("Failed to read awk output");

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);

            log::info!("Successfully evaluated awk expression with result: {}", result);

            return Some(result.to_string())
        } else {
            log::warn!("Failed to evaluate awk expression");
        }

    } else {
        log::warn!("Could not parse awk expresion: {}", expression);
    }

    None
}

fn sanitize_awk_expression(input: &str) -> Option<String> {
    let re = Regex::new(r"^awk\s*'([^']*)'$").expect("Failed to create regex");

    re.captures(input).and_then(|caps| {
        caps.get(1).map(|matched_text| matched_text.as_str().to_string())
    })
}
