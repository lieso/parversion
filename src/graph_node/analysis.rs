use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};

use super::{
    Graph, 
    GraphNodeData, 
    find_homologous_nodes,
    build_xml_with_target_node,
    get_depth,
    to_xml_string
};
use crate::xml_node::{XmlNode, get_meaningful_attributes};
use crate::basis_node::{BasisNode};
use crate::node_data_structure::{
    NodeDataStructure,
    EnumerativeStructure,
    AssociativeStructure
};
use crate::macros::*;
use crate::config::{CONFIG};
use crate::constants;
use crate::llm::{
    interpret_data_structure,
    interpret_element_data,
    interpret_text_data,
    interpret_associations
};
use crate::basis_graph::{Subgraph};

pub async fn analyze(
    target_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    subgraph: Subgraph,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
ANALYZING NODE DATA:
{}
Node:       {}
Hash:       {}
Lineage:    {}
{}",
            block_separator,
            block_separator,
            read_lock!(target_node).data.describe(),
            read_lock!(target_node).hash,
            read_lock!(target_node).lineage,
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

    analyze_data(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
        &subgraph,
    ).await;
}

pub async fn analyze_recursions(
    basis_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_recursions");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
ANALYZING NODE RECURSIONS:
{}
Node:       {}
Hash:       {}
Lineage:    {}
{}",
            block_separator,
            block_separator,
            read_lock!(basis_node).data.describe(),
            read_lock!(basis_node).hash,
            read_lock!(basis_node).lineage,
            block_separator,
        ));
    }

    let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
        Arc::clone(&basis_node),
        Arc::clone(&basis_root_node),
        Arc::clone(&output_tree),
    );

    if analyze_classically(Arc::clone(&basis_node), homologous_nodes.clone()) {
        log::info!("Basis node analyzed classically completely, not proceeding any further...");
        return;
    }

    analyze_structure(
        Arc::clone(&basis_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
}

pub async fn analyze_associations(
    basis_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_associations");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
ANALYZING NODE ASSOCIATIONS:
{}
Node:       {}
Hash:       {}
Lineage:    {}
{}",
            block_separator,
            block_separator,
            read_lock!(basis_node).data.describe(),
            read_lock!(basis_node).hash,
            read_lock!(basis_node).lineage,
            block_separator,
        ));
    }

    let mut snippets: Vec<(String, String)> = Vec::new();
    let target_node_parent: Graph<BasisNode> = read_lock!(basis_node).parents.first().unwrap().clone();
    let children: Vec<Graph<BasisNode>> = read_lock!(target_node_parent).children.clone();

    if children.len() < 2 {
        log::info!("Basis node does not have siblings");
        return;
    }

    for child in children.iter() {
        let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
            Arc::clone(&child),
            Arc::clone(&basis_root_node),
            Arc::clone(&output_tree),
        );

        let depths: Vec<usize> = homologous_nodes.iter()
            .map(|node| get_depth(Arc::clone(&node)))
            .collect();
        let max_depth = depths.iter().copied().max().unwrap_or(0);
        let deepest_nodes: Vec<Graph<XmlNode>> = homologous_nodes.iter()
            .filter(|node| get_depth(Arc::clone(node))  == max_depth)
            .cloned()
            .collect();

        let exemplary_nodes: Vec<(Graph<XmlNode>, String)> = deepest_nodes
            .into_iter()
            .take(10)
            .map(|item| {
                let hash = read_lock!(item).hash.clone();
                (item, hash)
            })
            .collect();

        for (exemplary_node, type_id) in exemplary_nodes.iter() {
            let xml_string = to_xml_string(Arc::clone(&exemplary_node));

            if xml_string.len() < 3000 {
                snippets.push((type_id.clone(), xml_string));
            }
        }
    }

    // Assumming that tiny snippets can never provide enough meaningful information
    // for associations to be determined
    snippets.retain(|(_, snippet_content)| snippet_content.len() > 200);

    if snippets.len() < 2 {
        log::info!("Did not receive enough snippets. Aborting...");
        return;
    }

    let interpretation = interpret_associations(snippets.clone()).await;

    if interpretation.is_empty() {
        log::info!("Snippets interpreted to be completely unrelated");
        return;
    }

    let associations = AssociativeStructure {
        subgraph_ids: interpretation,
    };

    let node_data_structure = NodeDataStructure {
        recursive: None,
        enumerative: None,
        associative: Some(associations),
    };

    for child in children.iter() {
        let read_lock = read_lock!(child);
        let mut write_lock = write_lock!(read_lock.data.structure);
        write_lock.push(node_data_structure.clone());
    }
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
    let tag_name = read_lock!(output_node).data.get_element_tag_name();

    if constants::SEEN_BLACKLISTED_ELEMENTS.contains(&tag_name.as_str()) {
        log::info!("Node with tag name: {} has been blacklisted from interpretation. Not proceeding any further.", tag_name);
        return true;
    }

    // The title of a document is reliably the child text node of the title element
    // So we won't need to ask an LLM for this type of information
    if read_lock!(target_node).hash == constants::TEXT_NODE_HASH {
        let binding = read_lock!(output_node);
        let parent = binding.parents.first().unwrap();
        let parent_tag_name = read_lock!(parent).data.get_element_tag_name();

        if parent_tag_name == "title" {
            log::info!("Text node is the document title. Not proceeding further");
            return true;
        }
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

    let target_node_examples_count = read_lock!(CONFIG)
        .llm
        .data_structure_interpretation
        .target_node_examples_max_count
        .clone();
    let target_node_examples_count = std::cmp::min(
        target_node_examples_count,
        homologous_nodes.len()
    );
    let target_node_adjacent_xml_length = read_lock!(CONFIG)
        .llm
        .data_structure_interpretation
        .target_node_adjacent_xml_length
        .clone();
    let snippets = make_snippets(
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
        target_node_examples_count,
        target_node_adjacent_xml_length
    );

    let recursive_structure = interpret_data_structure(snippets).await;
    let node_data_structure = NodeDataStructure {
        recursive: Some(recursive_structure),
        enumerative: None,
        associative: None,
    };

    {
        let rl = read_lock!(target_node);
        let mut wl = write_lock!(rl.data.structure);
        wl.push(node_data_structure);
    }
}

async fn analyze_data(
    target_node: Graph<BasisNode>, 
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>,
    subgraph: &Subgraph,
) {
    log::trace!("In analyze_data");

    if analyze_data_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node data analyzed classically, not proceeding any further...");
        return;
    }

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    let target_node_examples_count = read_lock!(CONFIG).llm.target_node_examples_max_count.clone();
    let target_node_examples_count = std::cmp::min(target_node_examples_count, homologous_nodes.len());
    let target_node_adjacent_xml_length = read_lock!(CONFIG).llm.target_node_adjacent_xml_length;
    let snippets = make_snippets(
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
        target_node_examples_count,
        target_node_adjacent_xml_length
    );

    if read_lock!(output_node).data.is_text() {
        let interpretation = interpret_text_data(snippets, subgraph.page_type.description.clone()).await;

        {
            let rl = read_lock!(target_node);
            let mut wl = write_lock!(rl.data.data);
            wl.push(interpretation);
        }
    } else {
        let meaningful_attributes: Vec<_> = get_meaningful_attributes(&read_lock!(output_node).data)
            .keys()
            .cloned()
            .filter(|key| {
                let is_empty_attribute = homologous_nodes
                    .iter()
                    .fold(true, |acc, node| {
                        let xml_node = read_lock!(node).data.clone();

                        acc && xml_node.get_attribute_value(key).unwrap().is_empty()
                    });

                !is_empty_attribute
            })
            .collect();

        if !meaningful_attributes.is_empty() {
            let interpretation = interpret_element_data(
                meaningful_attributes,
                snippets,
                subgraph.page_type.description.clone()
            ).await;

            {
                let rl = read_lock!(target_node);
                let mut wl = write_lock!(rl.data.data);
                wl.extend(interpretation);
            }
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

fn make_snippets(
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>,
    target_node_examples_count: usize,
    target_node_adjacent_xml_length: usize,
) -> Vec<String> {
    log::trace!("In make_snippets");
    log::info!("Using {} examples of target node for analysis", target_node_examples_count);
    
    let snippets: Vec<String> = homologous_nodes[..target_node_examples_count]
        .to_vec()
        .iter()
        .map(|item| node_to_snippet(Arc::clone(item), Arc::clone(&output_tree), target_node_adjacent_xml_length))
        .collect();

    snippets
}

fn analyze_structure_classically(basis_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_structure_classically");

    let exemplary_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();
    let output_parent_node: Option<Graph<XmlNode>> = read_lock!(exemplary_node).parents.first().cloned();

    if let Some(ref exemplary_parent) = output_parent_node {
        if homologous_nodes.len() > 1 {
            log::info!("Homologous node count is greater than one.");

            // Do all homologous nodes have the same parent?
            let are_siblings = homologous_nodes.iter().fold(true, |acc, node| {
                let parent = read_lock!(node).parents.first().cloned();
                let parent = parent.unwrap();

                acc && read_lock!(exemplary_parent).id == read_lock!(parent).id
            });
            log::debug!("are_siblings: {}", are_siblings);

            // If all homologous nodes have the same parent, that means this node represents a list of items of some kind
            if are_siblings {
                log::info!("Identified enumerative content");

                let enumerative_structure = EnumerativeStructure {
                    intrinsic_component_ids: vec![read_lock!(basis_node).id.clone()]
                };
                let node_data_structure = NodeDataStructure {
                    recursive: None,
                    enumerative: Some(enumerative_structure),
                    associative: None,
                };

                let binding = read_lock!(basis_node);
                let mut write_lock = write_lock!(binding.data.structure);
                write_lock.push(node_data_structure);
            }
        }
    }

    // Text nodes do not represent complex relationships
    if read_lock!(exemplary_node).data.is_text() {
        log::info!("Node is a text node. Not proceeding any further.");
        return true;
    }

    // Assuming nodes that are the lone child of their parent do not represent
    // any complex relationships to other nodes
    if let Some(ref parent) = output_parent_node {
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

fn node_to_snippet(
    node: Graph<XmlNode>,
    output_tree: Graph<XmlNode>,
    context_length: usize,
) -> String {
    log::trace!("In node_to_snippet");

    let document = build_xml_with_target_node(Arc::clone(&output_tree), Arc::clone(&node));

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
