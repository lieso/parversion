use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{
    GraphNode, 
    Graph, 
    GraphNodeData, 
    bft, 
    find_homologous_nodes,
    build_xml_with_target_node
};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::macros::*;
use crate::config::{CONFIG, Config};
use crate::constants;

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

    // * Basis root node
    if read_lock!(target_node).hash == constants::ROOT_NODE_HASH {
        log::info!("Node is root node, probably don't need to do anything here");
        return;
    }






    let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
        Arc::clone(&target_node),
        Arc::clone(&basis_root_node),
        Arc::clone(&output_tree),
    );
    //for node in homologous_nodes.iter() {
    //    log::debug!("homologous node: {}", read_lock!(node).data.describe());
    //}
    if homologous_nodes.is_empty() {
        panic!("There cannot be zero homologous nodes for any basis node with respect to output tree.");
    }





    analyze_structure(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
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

    let snippets = make_snippets(homologous_nodes.clone(), Arc::clone(&output_tree));
    log::debug!("snippet: {}", snippets.get(0).unwrap());

    unimplemented!()
}

fn make_snippets(homologous_nodes: Vec<Graph<XmlNode>>, output_tree: Graph<XmlNode>) -> Vec<String> {
    log::trace!("In make_snippets");

    let target_node_examples_max_count = read_lock!(CONFIG).llm.target_node_examples_max_count.clone();
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

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();
    let output_parent_node: Option<Graph<XmlNode>> = read_lock!(output_node).parents.first().cloned();

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
