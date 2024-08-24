use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{
    Graph, 
    GraphNodeData, 
    find_homologous_nodes,
    build_xml_with_target_node
};
use crate::xml_node::{XmlNode, get_meaningful_attributes};
use crate::basis_node::{BasisNode};
use crate::macros::*;
use crate::config::{CONFIG};
use crate::constants;
use crate::llm::{interpret_data_structure, interpret_element_data, interpret_text_data};

pub async fn analyze(
    target_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
ANALYZING NODE:
{}
Node:   {}
{}",
            block_separator,
            block_separator,
            read_lock!(target_node).data.describe(),
            block_separator,
        ));
    }

    let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
        Arc::clone(&target_node),
        Arc::clone(&basis_root_node),
        Arc::clone(&output_tree),
    );

    if analyze_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node analyzed classically completely, not proceeding any further...");
        return;
    }

    analyze_structure(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
    analyze_data(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
}

fn analyze_classically(target_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_classically");

    // * Basis root node
    if read_lock!(target_node).hash == constants::ROOT_NODE_HASH {
        log::info!("Node is root node, probably don't need to do anything here");
        return true;
    } else {
        if homologous_nodes.is_empty() {
            panic!("There cannot be zero homologous nodes for any basis node with respect to output tree.");
        }
    }

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    // * Link elements
    if read_lock!(output_node).data.get_element_tag_name() == "link" {
        log::info!("Node represents HTML link element. Not proceeding any further.");
        return true;
    }

    // * Meta elements
    if read_lock!(output_node).data.get_element_tag_name() == "meta" {
        log::info!("Node represents HTML meta element. Not proceeding any further.");
        return true;
    }

    // * Script elements
    if read_lock!(output_node).data.get_element_tag_name() == "script" {
        log::info!("Node represents HTML script element. Not proceeding any further.");
        return true;
    }

    // * Head elements
    if read_lock!(output_node).data.get_element_tag_name() == "head" {
        log::info!("Node represents HTML head element. Not proceeding any further.");
        return true;
    }

    // * Body elements
    if read_lock!(output_node).data.get_element_tag_name() == "body" {
        log::info!("Node represents HTML body element. Not proceeding any further.");
        return true;
    }

    // * br elements
    if read_lock!(output_node).data.get_element_tag_name() == "br" {
        log::info!("Node represents HTML break element. Not proceeding any further.");
        return true;
    }

    // * form elements
    if read_lock!(output_node).data.get_element_tag_name() == "form" {
        log::info!("Node represents HTML form element. Not proceeding any further.");
        return true;
    }

    false
}

async fn analyze_structure(
    target_node: Graph<BasisNode>, 
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>
) {
    log::trace!("In analyze_structure");

    if analyze_structure_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node structure analyzed classically, not proceeding any further...");
        return;
    }

    let snippets = make_snippets(homologous_nodes.clone(), Arc::clone(&output_tree), true);

    let structure = interpret_data_structure(snippets).await;

    {
        let rl = read_lock!(target_node);
        let mut wl = write_lock!(rl.data.structure);
        wl.push(structure);
    }
}

async fn analyze_data(
    target_node: Graph<BasisNode>, 
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>
) {
    log::trace!("In analyze_data");

    if analyze_data_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node data analyzed classically, not proceeding any further...");
        return;
    }

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();
    let snippets = make_snippets(homologous_nodes.clone(), Arc::clone(&output_tree), false);

    if read_lock!(output_node).data.is_text() {
        let interpretation = interpret_text_data(snippets).await;

        {
            let rl = read_lock!(target_node);
            let mut wl = write_lock!(rl.data.data);
            wl.push(interpretation);
        }
    } else {

        let meaningful_attributes = get_meaningful_attributes(&read_lock!(output_node).data)
            .keys()
            .cloned()
            .collect();

        let interpretation = interpret_element_data(meaningful_attributes, snippets).await;

        {
            let rl = read_lock!(target_node);
            let mut wl = write_lock!(rl.data.data);
            wl.extend(interpretation);
        }
    }
}

fn analyze_data_classically(_basis_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_data_classically");

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    if read_lock!(output_node).data.is_element() {
        let meaningful_attributes = get_meaningful_attributes(&read_lock!(output_node).data);

        if meaningful_attributes.is_empty() {
            log::info!("Node represents HTML element without any meaningful attributes. Not proceeding any further.");

            return true;
        }
    }

    false
}

fn make_snippets(homologous_nodes: Vec<Graph<XmlNode>>, output_tree: Graph<XmlNode>, extend: bool) -> Vec<String> {
    log::trace!("In make_snippets");

    let mut target_node_examples_max_count = read_lock!(CONFIG).llm.target_node_examples_max_count.clone();
    if extend { target_node_examples_max_count = target_node_examples_max_count + 5 };
    let target_node_examples_count = std::cmp::min(target_node_examples_max_count, homologous_nodes.len());
    log::info!("Using {} examples of target node for analysis", target_node_examples_count);
    
    let snippets: Vec<String> = homologous_nodes[..target_node_examples_count]
        .to_vec()
        .iter()
        .map(|item| node_to_snippet(Arc::clone(item), Arc::clone(&output_tree)))
        .collect();

    snippets
}

fn analyze_structure_classically(basis_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_structure_classically");

    let exemplary_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();
    let output_parent_node: Option<Graph<XmlNode>> = read_lock!(exemplary_node).parents.first().cloned();

    // It's unlikely that there are complex relationships for nodes that only appear a couple times in a document
    if homologous_nodes.len() < 3 {
        log::info!("Homologous node count is less than three. Not proceeding any further.");
        return true;
    }

    // Text nodes do not represent complex relationships
    if read_lock!(exemplary_node).data.is_text() {
        log::info!("Node is a text node. Not proceeding any further.");
        return true;
    }

    // Assuming nodes that are the lone child of their parent do not represent
    // any complex relationships to other nodes
    if let Some(parent) = output_parent_node {
        let parent_out_degree = read_lock!(parent).children.len();

        if parent_out_degree < 2 {
            log::info!("Node parent has out-degree less than two. Not proceeding any further.");
            return true;
        }
    } else {
        log::info!("Output node is root node. Not proceeding any further.");
        return true;
    }

    // If a basis node is part of a cycle, it represents a recursive relationship
    // in the underlying data model 
    let parent_count = read_lock!(basis_node).parents.len();
    if parent_count > 1 {
        log::info!("Node has more than one parent and is therefore recursive. Not proceeding any further.");
        return true;
    }

    false
}

fn node_to_snippet(node: Graph<XmlNode>, output_tree: Graph<XmlNode>) -> String {
    log::trace!("In node_to_snippet");

    let document = build_xml_with_target_node(Arc::clone(&output_tree), Arc::clone(&node));
    let context_length = read_lock!(CONFIG).llm.target_node_adjacent_xml_length;

    if read_lock!(node).data.is_text() {
        format!(
            "{}<!--Target node start -->{}<!--Target node end -->{}",
            take_from_end(&document.0, context_length),
            document.2,
            take_from_start(&document.4, context_length),
        )
    } else {
        let after_start_tag = &format!(
            "{}{}{}",
            document.2,
            document.3,
            document.4
        );

        format!(
            "{}<!--Target node start -->{}<!--Target node end -->{}",
            take_from_end(&document.0, context_length),
            document.1,
            take_from_start(after_start_tag, context_length),
        )
    }
}

fn take_from_end(s: &str, amount: usize) -> &str {
    log::trace!("In take_from_end");

    let len = s.len();
    if amount >= len {
        s
    } else {
        let start_index = len - amount;
        let mut adjusted_start = start_index;

        while !s.is_char_boundary(adjusted_start) && adjusted_start < len {
            adjusted_start += 1;
        }

        &s[adjusted_start..]
    }
}

fn take_from_start(s: &str, amount: usize) -> &str {
    log::trace!("In take_from_end");

    if amount >= s.len() {
        s
    } else {
        let end_index = amount;
        let mut adjusted_end = end_index;

        while !s.is_char_boundary(adjusted_end) && adjusted_end > 0 {
            adjusted_end -= 1;
        }

        &s[..adjusted_end]
    }
}
