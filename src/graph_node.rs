use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::basis_node::BasisNode;
use crate::data_node::DataNode;
use crate::prelude::*;
use crate::xpath::{XPath, XPathAxis, XPathPredicate, XPathSegment};

pub type Graph = Arc<RwLock<GraphNode>>;
pub type GraphNodeID = ID;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode {
    pub id: ID,
    pub parents: Vec<Graph>,
    pub description: String,
    pub hash: Hash,
    pub subgraph_hash: Hash,
    pub lineage: Lineage,
    pub children: Vec<Graph>,
}

impl GraphNode {
    pub fn from_data_node(data_node: Arc<DataNode>, parents: Vec<Graph>) -> Self {
        let hash = data_node.hash.clone();

        GraphNode {
            id: ID::new(),
            parents,
            description: data_node.description.clone(),
            hash: hash.clone(),
            subgraph_hash: hash.clone(),
            lineage: data_node.lineage.clone(),
            children: Vec::new(),
        }
    }

    pub fn preorder_position(&self) -> usize {
        let mut position = 0;
        let mut current = Some(self.clone());
        let mut ancestors = Vec::new();

        while let Some(node) = current {
            ancestors.push(node.clone());
            current = node
                .parents
                .first()
                .map(|parent| read_lock!(parent).clone());
        }

        ancestors.reverse();

        for ancestor in ancestors {
            position = position * ancestor.children.len() + ancestor.index_in_parent().unwrap_or(0);
        }

        position
    }

    pub fn index_in_parent(&self) -> Option<usize> {
        self.parents.first().and_then(|parent| {
            read_lock!(parent)
                .children
                .iter()
                .position(|child| read_lock!(child).id == self.id)
        })
    }

    pub fn index_in_parent_by_type(&self, meta_context: &MetaContext) -> Option<usize> {
        self.parents.first().and_then(|parent| {
            let parent_lock = read_lock!(parent);
            let self_context = meta_context.contexts_lookup.get(&self.id)?;
            let self_element_name = read_lock!(self_context.document_node).get_element_name();

            let same_type_siblings: Vec<_> = parent_lock
                .children
                .iter()
                .filter(|child| {
                    let child_lock = read_lock!(child);
                    if let Some(child_context) = meta_context.contexts_lookup.get(&child_lock.id) {
                        let child_element_name =
                            read_lock!(child_context.document_node).get_element_name();
                        child_element_name == self_element_name
                    } else {
                        false
                    }
                })
                .collect();

            same_type_siblings
                .iter()
                .position(|child| read_lock!(child).id == self.id)
        })
    }

    pub fn subgraph_hash(&self) -> Hash {
        let mut combined_hash = Hash::new();

        combined_hash.push(self.hash.to_string().unwrap_or_default());

        for child in &self.children {
            let child_read = read_lock!(child);
            let child_subgraph_hash = child_read.subgraph_hash();
            combined_hash.push(child_subgraph_hash.to_string().unwrap_or_default());
        }

        combined_hash.sort();
        combined_hash.finalize();

        combined_hash
    }

    pub fn acyclic_subgraph_hash(&self) -> Hash {
        let mut combined_hash = Hash::new();

        combined_hash.push(self.hash.to_string().unwrap_or_default());

        for child in &self.children {
            let child_read = read_lock!(child);
            if child_read.lineage.is_cyclic() {
                continue;
            }
            let child_subgraph_hash = child_read.acyclic_subgraph_hash();
            combined_hash.push(child_subgraph_hash.to_string().unwrap_or_default());
        }

        combined_hash.sort();
        combined_hash.finalize();

        combined_hash
    }

    pub fn get_indexed_lineage_at_depth(&self, target_depth: usize) -> Option<Lineage> {
        let mut ancestors = Vec::new();

        ancestors.push((self.id.clone(), self.hash.clone(), self.index_in_parent()));

        let mut remaining_parents = self.parents.clone();
        while !remaining_parents.is_empty() {
            let parent = read_lock!(remaining_parents[0]).clone();
            ancestors.push((
                parent.id.clone(),
                parent.hash.clone(),
                parent.index_in_parent(),
            ));
            remaining_parents = parent.parents.clone();
        }

        if target_depth >= ancestors.len() {
            return None;
        }

        let mut lineage = Lineage::new();

        for (depth, (_, hash, index)) in ancestors.iter().enumerate() {
            if depth == target_depth {
                if let Some(idx) = index {
                    lineage = lineage.with_hash(Hash::from_str(&idx.to_string()));
                }
            }
            lineage = lineage.with_hash(hash.clone());
        }

        Some(lineage)
    }

    pub fn resolve_basis_node(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
    ) -> Result<Option<Arc<BasisNode>>, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        let context_to_group = {
            let lock = read_lock!(normalization_context);
            lock.context_to_group
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "'context_to_group' not provided in normalization context".to_string(),
                ))?
        };

        let context = meta_context.contexts_lookup.get(&self.id).cloned().unwrap();

        if let Some(basis_group) = context_to_group.get(&context.id).cloned() {
            let basis_lineage = basis_group.get_basis_lineage();
            let basis_node: Arc<BasisNode> = {
                let lock = read_lock!(normalization_context);
                lock.get_basis_node_by_lineage(&basis_lineage)
                    .expect("Could not get basis node by lineage")
                    .expect("basis group resolved but no basis node exists for its lineage")
            };

            Ok(Some(basis_node))
        } else {
            Ok(None)
        }
    }
}

impl GraphNode {
    pub fn traverse_using_xpath_axis(
        _meta_context: Arc<RwLock<NormalizationContext>>,
        graph: Graph,
        xpath_axis: &XPathAxis,
    ) -> Result<Vec<Graph>, Errors> {
        let lock = read_lock!(graph);

        log::warn!("===== XPATH AXIS TRAVERSAL =====");
        log::warn!("XPATH AXIS: {:?}", xpath_axis);
        log::warn!("Current node ID: {}", lock.id.to_string());
        log::warn!("Current node has {} children, {} parents", lock.children.len(), lock.parents.len());

        if lock.parents.len() > 1 {
            log::error!("ERROR: Node has multiple parents ({}) - xpath requires single parent", lock.parents.len());
            return Err(Errors::XPathTraverseError(
                "Why are we traversing a graph using xpath if nodes have more than one parent?"
                    .to_string(),
            ));
        }

        let result = match xpath_axis {
            XPathAxis::Child => {
                log::info!("Applying XPATH AXIS::Child - returning {} children", lock.children.len());
                for (idx, child) in lock.children.iter().enumerate() {
                    log::debug!("  Child {}: {}", idx, read_lock!(child).id.to_string());
                }
                Ok(lock.children.clone())
            },
            XPathAxis::Parent => {
                log::info!("Applying XPATH AXIS::Parent - returning {} parents", lock.parents.len());
                for (idx, parent) in lock.parents.iter().enumerate() {
                    log::debug!("  Parent {}: {}", idx, read_lock!(parent).id.to_string());
                }
                Ok(lock.parents.clone())
            },
            XPathAxis::Attribute => {
                log::info!("Applying XPATH AXIS::Attribute - staying on current node");
                Ok(vec![Arc::clone(&graph)])
            },
            XPathAxis::Self_ => {
                log::info!("Applying XPATH AXIS::Self_ - returning current node");
                Ok(vec![graph.clone()])
            },
            XPathAxis::Descendant => {
                log::info!("Applying XPATH AXIS::Descendant - traversing all descendants");
                let mut descendants = Vec::new();
                let mut queue = lock.children.clone();
                log::debug!("Starting with {} children", queue.len());

                let mut depth = 0;
                while !queue.is_empty() {
                    let node = queue.remove(0);
                    let node_lock = read_lock!(node);
                    descendants.push(node.clone());
                    log::debug!("  [Descendant depth {}] Added node {}, has {} children", depth, node_lock.id.to_string(), node_lock.children.len());
                    queue.extend(node_lock.children.clone());
                    depth += 1;
                }

                log::info!("XPATH AXIS::Descendant - found {} total descendants", descendants.len());
                Ok(descendants)
            },
            XPathAxis::Ancestor => {
                log::info!("Applying XPATH AXIS::Ancestor - traversing all ancestors");
                let mut ancestors = Vec::new();
                let mut current_parents = lock.parents.clone();
                let mut depth = 0;

                while !current_parents.is_empty() {
                    let parent = current_parents[0].clone();
                    let parent_lock = read_lock!(parent);
                    ancestors.push(parent.clone());
                    log::debug!("  [Ancestor depth {}] Added ancestor {}, has {} parents", depth, parent_lock.id.to_string(), parent_lock.parents.len());
                    current_parents = parent_lock.parents.clone();
                    depth += 1;
                }

                log::info!("XPATH AXIS::Ancestor - found {} total ancestors", ancestors.len());
                Ok(ancestors)
            },
            XPathAxis::FollowingSibling => {
                log::info!("Applying XPATH AXIS::FollowingSibling");
                if let Some(parent) = lock.parents.first() {
                    let parent_lock = read_lock!(parent);
                    if let Some(index_current) = parent_lock
                        .children
                        .iter()
                        .position(|child| read_lock!(child).id == lock.id)
                    {
                        let siblings: Vec<Graph> =
                            parent_lock.children[index_current + 1..].to_vec();
                        log::info!("XPATH AXIS::FollowingSibling - current at index {}, found {} following siblings", index_current, siblings.len());
                        Ok(siblings)
                    } else {
                        log::error!("XPATH AXIS::FollowingSibling - Could not find current node in parent's children");
                        Err(Errors::XPathTraverseError(
                            "Could not find index of current node as a child of parent".to_string(),
                        ))
                    }
                } else {
                    log::error!("XPATH AXIS::FollowingSibling - No parent found (root node)");
                    Err(Errors::XPathTraverseError(
                        "Trying to visit following sibling on a root node".to_string(),
                    ))
                }
            },
            XPathAxis::PrecedingSibling => {
                log::info!("Applying XPATH AXIS::PrecedingSibling");
                if let Some(parent) = lock.parents.first() {
                    let parent_lock = read_lock!(parent);

                    if let Some(index_current) = parent_lock
                        .children
                        .iter()
                        .position(|child| read_lock!(child).id == lock.id)
                    {
                        let siblings: Vec<Graph> = parent_lock.children[..index_current]
                            .iter()
                            .rev()
                            .cloned()
                            .collect();
                        log::info!("XPATH AXIS::PrecedingSibling - current at index {}, found {} preceding siblings", index_current, siblings.len());

                        // Get context for current node and log it
                        let meta_context_ref = {
                            let norm_lock = read_lock!(_meta_context);
                            norm_lock.meta_context.as_ref().unwrap().clone()
                        };
                        let contexts_lookup = meta_context_ref.contexts_lookup.clone();
                        if let Some(context) = contexts_lookup.get(&lock.id) {
                            match context.generate_context_string(&meta_context_ref, Vec::new()) {
                                Ok(context_string) => {
                                    log::info!("XPATH AXIS::PrecedingSibling - Current node context:\n{}", context_string);
                                }
                                Err(e) => {
                                    log::error!("XPATH AXIS::PrecedingSibling - Error generating context string: {:?}", e);
                                }
                            }
                        }

                        Ok(siblings)
                    } else {
                        log::error!("XPATH AXIS::PrecedingSibling - Could not find current node in parent's children");
                        Err(Errors::XPathTraverseError(
                            "Could not find index of current node as a child of parent".to_string(),
                        ))
                    }
                } else {
                    log::error!("XPATH AXIS::PrecedingSibling - No parent found (root node)");
                    Err(Errors::XPathTraverseError(
                        "Trying to visit preceding sibling on a root node".to_string(),
                    ))
                }
            },
            XPathAxis::Following => {
                log::info!("Applying XPATH AXIS::Following - collecting all following nodes");
                let mut result = Vec::new();
                let mut current_id = lock.id.clone();
                let mut current_parents = lock.parents.clone();
                let mut iteration = 0;

                loop {
                    let Some(parent) = current_parents.first().cloned() else {
                        log::debug!("  [Following iteration {}] Reached root (no parent)", iteration);
                        break;
                    };

                    let (next_id, next_parents, following_siblings) = {
                        let parent_lock = read_lock!(parent);
                        let Some(index) = parent_lock
                            .children
                            .iter()
                            .position(|child| read_lock!(child).id == current_id)
                        else {
                            log::error!("XPATH AXIS::Following - Could not find current node in parent's children");
                            return Err(Errors::XPathTraverseError(
                                "Could not find index of current node as a child of parent"
                                    .to_string(),
                            ));
                        };
                        let following_siblings = parent_lock.children[index + 1..].to_vec();
                        log::debug!("  [Following iteration {}] At parent {}, found {} following siblings at indices {}..{}",
                                   iteration, parent_lock.id.to_string(), following_siblings.len(), index + 1, parent_lock.children.len());
                        (
                            parent_lock.id.clone(),
                            parent_lock.parents.clone(),
                            following_siblings,
                        )
                    };

                    for sibling in following_siblings {
                        let sibling_lock = read_lock!(sibling);
                        result.push(sibling.clone());
                        log::trace!("    [Following] Added sibling {}", sibling_lock.id.to_string());
                        let mut queue = sibling_lock.children.clone();
                        while !queue.is_empty() {
                            let desc = queue.remove(0);
                            let desc_lock = read_lock!(desc);
                            result.push(desc.clone());
                            log::trace!("    [Following] Added descendant {}", desc_lock.id.to_string());
                            queue.extend(desc_lock.children.clone());
                        }
                    }

                    current_id = next_id;
                    current_parents = next_parents;
                    iteration += 1;
                }

                log::info!("XPATH AXIS::Following - collected {} total following nodes", result.len());
                Ok(result)
            },
            XPathAxis::Preceding => {
                log::info!("Applying XPATH AXIS::Preceding - collecting all preceding nodes");
                let mut result = Vec::new();
                let mut current_id = lock.id.clone();
                let mut current_parents = lock.parents.clone();
                let mut iteration = 0;

                loop {
                    let Some(parent) = current_parents.first().cloned() else {
                        log::debug!("  [Preceding iteration {}] Reached root (no parent)", iteration);
                        break;
                    };

                    let (next_id, next_parents, preceding_siblings) = {
                        let parent_lock = read_lock!(parent);
                        let Some(index) = parent_lock
                            .children
                            .iter()
                            .position(|child| read_lock!(child).id == current_id)
                        else {
                            log::error!("XPATH AXIS::Preceding - Could not find current node in parent's children");
                            return Err(Errors::XPathTraverseError(
                                "Could not find index of current node as a child of parent"
                                    .to_string(),
                            ));
                        };
                        let preceding_siblings: Vec<Graph> = parent_lock.children[..index]
                            .iter()
                            .rev()
                            .cloned()
                            .collect();
                        log::debug!("  [Preceding iteration {}] At parent {}, found {} preceding siblings at indices 0..{}",
                                   iteration, parent_lock.id.to_string(), preceding_siblings.len(), index);
                        (
                            parent_lock.id.clone(),
                            parent_lock.parents.clone(),
                            preceding_siblings,
                        )
                    };

                    for sibling in preceding_siblings {
                        let sibling_lock = read_lock!(sibling);
                        result.push(sibling.clone());
                        log::trace!("    [Preceding] Added sibling {}", sibling_lock.id.to_string());
                        let mut queue = sibling_lock.children.clone();
                        while !queue.is_empty() {
                            let desc = queue.remove(0);
                            let desc_lock = read_lock!(desc);
                            result.push(desc.clone());
                            log::trace!("    [Preceding] Added descendant {}", desc_lock.id.to_string());
                            queue.extend(desc_lock.children.clone());
                        }
                    }

                    current_id = next_id;
                    current_parents = next_parents;
                    iteration += 1;
                }

                log::info!("XPATH AXIS::Preceding - collected {} total preceding nodes", result.len());
                Ok(result)
            },
        };

        log::warn!("===== END XPATH AXIS TRAVERSAL =====");
        result
    }

    pub fn traverse_using_xpath_node_test(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graph: Graph,
        node_test: &String,
    ) -> Result<Vec<Graph>, Errors> {
        let graph_id = read_lock!(graph).id.clone();

        log::warn!("===== XPATH NODE TEST =====");
        log::warn!("NODE TEST: '{}'", node_test);
        log::warn!("Current node ID: {}", graph_id.to_string());

        let contexts_lookup = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
        };
        if let Some(context) = contexts_lookup.get(&graph_id) {
            let doc_node = read_lock!(context.document_node);
            log::warn!("  DocumentNode: {}", doc_node.to_string());
        }

        if node_test == "node()" {
            log::error!("XPATH NODE TEST ERROR: Received forbidden node_test 'node()'");
            panic!("Received node_test 'node()'");
        }

        if node_test == "comment()" {
            log::error!("XPATH NODE TEST ERROR: Received forbidden node_test 'comment()'");
            panic!("Received node_test 'comment()'");
        }

        if node_test == "*" {
            log::error!("XPATH NODE TEST ERROR: Received forbidden node_test '*'");
            panic!("Received node_test '*'");
        }

        let node_test = if node_test == "text()" {
            log::info!("XPATH NODE TEST: Converting 'text()' to '#text'");
            "#text"
        } else {
            log::info!("XPATH NODE TEST: Using literal node test '{}'", node_test);
            node_test.as_str()
        };

        let current_context = contexts_lookup.get(&graph_id).unwrap();
        let document_node = current_context.document_node.clone();
        let name = read_lock!(document_node).get_element_name();

        log::info!("XPATH NODE TEST: Comparing node_test='{}' (trimmed) against element name='{}' (trimmed)",
                  node_test.trim(), name.trim());

        let result = if node_test.trim() == name.trim() {
            log::info!("XPATH NODE TEST: MATCH - node_test matches element name, returning current node");
            Ok(vec![graph.clone()])
        } else {
            log::info!("XPATH NODE TEST: NO MATCH - node_test '{}' != element name '{}'", node_test.trim(), name.trim());
            Ok(vec![])
        };

        log::warn!("===== END XPATH NODE TEST =====");
        result
    }

    pub fn traverse_using_xpath_predicate(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graphs: Vec<Graph>,
        predicate: &XPathPredicate,
    ) -> Result<Vec<Graph>, Errors> {
        log::warn!("===== XPATH PREDICATE =====");
        log::warn!("PREDICATE: {:?}", predicate);
        log::warn!("Input graph count: {}", graphs.len());

        let result = match predicate {
            XPathPredicate::Position(index) => {
                log::info!("XPATH PREDICATE::Position - filtering for position {}", index);
                // XPath positions are 1-indexed
                if *index < 1 || *index as usize > graphs.len() {
                    log::info!("XPATH PREDICATE::Position {} - OUT OF BOUNDS (graphs.len={}), returning empty", index, graphs.len());
                    return Ok(vec![]);
                }

                let selected_graph = graphs.get(*index as usize - 1).cloned().unwrap();
                log::info!("XPATH PREDICATE::Position {} - MATCH found, selecting node {}", index, read_lock!(selected_graph).id.to_string());
                Ok(vec![selected_graph])
            },
            XPathPredicate::Last => {
                log::info!("XPATH PREDICATE::Last - selecting last graph from {} graphs", graphs.len());
                let result = graphs.last().cloned().into_iter().collect();
                if let Some(last_graph) = graphs.last() {
                    log::info!("XPATH PREDICATE::Last - selected node {}", read_lock!(last_graph).id.to_string());
                }
                Ok(result)
            },
            XPathPredicate::Not(inner) => {
                log::info!("XPATH PREDICATE::Not - applying inner predicate to filter");
                let mut filtered = Vec::new();
                let mut matched_count = 0;
                for graph in graphs {
                    let matched = Self::traverse_using_xpath_predicate(
                        Arc::clone(&normalization_context),
                        vec![Arc::clone(&graph)],
                        inner,
                    )?;
                    if matched.is_empty() {
                        filtered.push(graph);
                    } else {
                        matched_count += 1;
                    }
                }
                log::info!("XPATH PREDICATE::Not - filtered {} matched, kept {} unmatched", matched_count, filtered.len());
                Ok(filtered)
            },
            XPathPredicate::ContainsNormalized { value } => {
                log::info!("XPATH PREDICATE::ContainsNormalized - filtering for normalized text containing '{}'", value);
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        if let Some(context) = contexts_lookup.get(&graph_id) {
                            let text_vals = context.data_node.fields.get("text");
                            if !text_vals.is_empty() {
                                let text_str = text_vals[0].to_string();
                                let normalized =
                                    text_str.split_whitespace().collect::<Vec<_>>().join(" ");
                                let matches = normalized.contains(value.trim());
                                if matches {
                                    log::debug!("XPATH PREDICATE::ContainsNormalized - MATCH: '{}' contains '{}'", normalized, value.trim());
                                    matched_count += 1;
                                }
                                return matches;
                            }
                            false
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();
                log::info!("XPATH PREDICATE::ContainsNormalized - matched {} out of {} graphs", matched_count, graphs.len());
                Ok(filtered)
            },
            XPathPredicate::Contains { name, value } => {
                log::info!("XPATH PREDICATE::Contains - filtering for attribute '{}' containing '{}'", name, value);
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        let matches = contexts_lookup
                            .get(&graph_id)
                            .and_then(|context| {
                                let doc_node = read_lock!(&context.document_node);
                                log::trace!("XPATH PREDICATE::Contains - checking node {} ({})", graph_id.to_string(), doc_node.to_string());
                                doc_node
                                    .get_attribute_value(name)
                                    .map(|attr_value| attr_value.trim().contains(value.trim()))
                            })
                            .unwrap_or(false);
                        if matches {
                            if let Some(context) = contexts_lookup.get(&graph_id) {
                                let doc_node = read_lock!(&context.document_node);
                                log::debug!("XPATH PREDICATE::Contains - MATCH on node {} ({})", graph_id.to_string(), doc_node.to_string());
                            }
                            matched_count += 1;
                        }
                        matches
                    })
                    .cloned()
                    .collect();

                log::info!("XPATH PREDICATE::Contains - matched {} out of {} graphs", matched_count, graphs.len());
                Ok(filtered)
            },
            XPathPredicate::Attribute { name, value } => {
                log::info!("XPATH PREDICATE::Attribute - filtering for exact attribute match: {}='{}'", name, value);
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        let matches = contexts_lookup
                            .get(&graph_id)
                            .and_then(|context| {
                                let doc_node = read_lock!(&context.document_node);
                                log::trace!("XPATH PREDICATE::Attribute - checking node {} ({})", graph_id.to_string(), doc_node.to_string());
                                doc_node
                                    .get_attribute_value(name)
                                    .map(|attr_value| attr_value.trim() == value.trim())
                            })
                            .unwrap_or(false);
                        if matches {
                            if let Some(context) = contexts_lookup.get(&graph_id) {
                                let doc_node = read_lock!(&context.document_node);
                                log::debug!("XPATH PREDICATE::Attribute - MATCH on node {} ({}, {}='{}')",
                                           graph_id.to_string(), doc_node.get_element_name(), name, value);
                            }
                            matched_count += 1;
                        }
                        matches
                    })
                    .cloned()
                    .collect();

                log::info!("XPATH PREDICATE::Attribute - matched {} out of {} graphs", matched_count, graphs.len());
                Ok(filtered)
            },
            XPathPredicate::AttributePresence(names) => {
                log::info!("XPATH PREDICATE::AttributePresence - filtering for presence of {} attributes: {:?}", names.len(), names);
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        let matches = contexts_lookup
                            .get(&graph_id)
                            .map(|context| {
                                let doc_node = read_lock!(&context.document_node);
                                log::trace!("XPATH PREDICATE::AttributePresence - checking node {} ({})", graph_id.to_string(), doc_node.to_string());
                                names.iter().all(|name| {
                                    doc_node
                                        .get_attribute_value(name)
                                        .is_some()
                                })
                            })
                            .unwrap_or(false);
                        if matches {
                            if let Some(context) = contexts_lookup.get(&graph_id) {
                                let doc_node = read_lock!(&context.document_node);
                                log::debug!("XPATH PREDICATE::AttributePresence - MATCH on node {} ({})", graph_id.to_string(), doc_node.get_element_name());
                            }
                            matched_count += 1;
                        }
                        matches
                    })
                    .cloned()
                    .collect();

                log::info!("XPATH PREDICATE::AttributePresence - matched {} out of {} graphs", matched_count, graphs.len());
                Ok(filtered)
            },
            XPathPredicate::StartsWith { name, value } => {
                log::info!("XPATH PREDICATE::StartsWith - filtering for attribute '{}' starting with '{}'", name, value);
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        let matches = contexts_lookup
                            .get(&graph_id)
                            .and_then(|context| {
                                let doc_node = read_lock!(&context.document_node);
                                log::trace!("XPATH PREDICATE::StartsWith - checking node {} ({})", graph_id.to_string(), doc_node.to_string());
                                doc_node
                                    .get_attribute_value(name)
                                    .map(|attr_value| attr_value.trim().starts_with(value.trim()))
                            })
                            .unwrap_or(false);
                        if matches {
                            if let Some(context) = contexts_lookup.get(&graph_id) {
                                let doc_node = read_lock!(&context.document_node);
                                log::debug!("XPATH PREDICATE::StartsWith - MATCH on node {} ({})", graph_id.to_string(), doc_node.get_element_name());
                            }
                            matched_count += 1;
                        }
                        matches
                    })
                    .cloned()
                    .collect();

                log::info!("XPATH PREDICATE::StartsWith - matched {} out of {} graphs", matched_count, graphs.len());
                Ok(filtered)
            },
            XPathPredicate::Path(path) => {
                log::info!("XPATH PREDICATE::Path - filtering based on path traversal");
                let mut matched_count = 0;
                let filtered: Vec<Graph> = graphs
                    .into_iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        let path_match = matches!(
                            Self::traverse_using_xpath(
                                Arc::clone(&normalization_context),
                                Arc::clone(graph),
                                path
                            ),
                            Ok(Some(_))
                        );
                        if path_match {
                            log::debug!("XPATH PREDICATE::Path - MATCH on node {}", graph_id.to_string());
                            matched_count += 1;
                        }
                        path_match
                    })
                    .collect();

                log::info!("XPATH PREDICATE::Path - matched {} graphs via path traversal", matched_count);
                Ok(filtered)
            },
            XPathPredicate::And(predicates) => {
                log::info!("XPATH PREDICATE::And - applying {} predicates sequentially", predicates.len());
                predicates.iter().try_fold(graphs, |acc, predicate| {
                    log::debug!("XPATH PREDICATE::And - applying predicate to {} graphs", acc.len());
                    Self::traverse_using_xpath_predicate(
                        Arc::clone(&normalization_context),
                        acc,
                        predicate,
                    )
                })
            }
        };

        log::warn!("===== END XPATH PREDICATE =====");
        result
    }

    pub fn traverse_using_xpath_segment(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graph: Graph,
        xpath_segment: &XPathSegment,
    ) -> Result<Vec<Graph>, Errors> {
        let graph_id = read_lock!(graph).id.clone();

        log::warn!("===== XPATH SEGMENT =====");
        log::warn!("SEGMENT - axis: {:?}, node_test: '{}', predicates: {}",
                  xpath_segment.axis, xpath_segment.node_test, xpath_segment.predicates.len());
        log::warn!("Starting node ID: {}", graph_id.to_string());

        let contexts_lookup = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
        };
        if let Some(context) = contexts_lookup.get(&graph_id) {
            let doc_node = read_lock!(context.document_node);
            log::warn!("  DocumentNode: {}", doc_node.to_string());
        }

        let mut next_graphs: Vec<Graph> = Self::traverse_using_xpath_axis(
            Arc::clone(&normalization_context),
            Arc::clone(&graph),
            &xpath_segment.axis,
        )?;

        log::info!("XPATH SEGMENT - after axis '{}', have {} graphs",
                  format!("{:?}", xpath_segment.axis), next_graphs.len());

        let mut next_graphs: Vec<Graph> =
            if matches!(xpath_segment.axis, XPathAxis::Self_ | XPathAxis::Parent | XPathAxis::Attribute) {
                log::info!("XPATH SEGMENT - skipping node_test for Self_/Parent axis");
                next_graphs
            } else {
                log::info!("XPATH SEGMENT - applying node_test '{}' to {} graphs", xpath_segment.node_test, next_graphs.len());
                let tested: Vec<Vec<Graph>> = next_graphs
                    .iter()
                    .map(|graph| {
                        Self::traverse_using_xpath_node_test(
                            Arc::clone(&normalization_context),
                            Arc::clone(&graph),
                            &xpath_segment.node_test,
                        )
                    })
                    .collect::<Result<Vec<Vec<Graph>>, Errors>>()?;

                let flattened: Vec<Graph> = tested.into_iter().flatten().collect();
                log::info!("XPATH SEGMENT - after node_test, have {} graphs", flattened.len());
                flattened
            };

        log::info!("XPATH SEGMENT - applying {} predicates", xpath_segment.predicates.len());
        let mut predicate_count = 0;
        let result = xpath_segment
                .predicates
                .iter()
                .try_fold(next_graphs, |graphs, predicate| {
                    predicate_count += 1;
                    log::debug!("XPATH SEGMENT - predicate {}/{}: {} graphs before", predicate_count, xpath_segment.predicates.len(), graphs.len());
                    let result = Self::traverse_using_xpath_predicate(
                        Arc::clone(&normalization_context),
                        graphs,
                        predicate,
                    );
                    if let Ok(ref filtered) = result {
                        log::debug!("XPATH SEGMENT - predicate {}/{}: {} graphs after", predicate_count, xpath_segment.predicates.len(), filtered.len());
                    }
                    result
                })?;

        log::info!("XPATH SEGMENT - final result: {} graphs", result.len());
        log::warn!("===== END XPATH SEGMENT =====");
        Ok(result)
    }

    pub fn traverse_using_xpath(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
        xpath: &XPath,
    ) -> Result<Option<Graph>, Errors> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let traversal_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() % 100000)
            .unwrap_or(0);

        log::error!("");
        log::error!("╔════════════════════════════════════════════════════════════════════════════════╗");
        log::error!("║                    ▶ XPATH TRAVERSAL START [ID: {}]                          ║", traversal_id);
        log::error!("╚════════════════════════════════════════════════════════════════════════════════╝");
        log::error!("[{}] Starting node ID: {}", traversal_id, read_lock!(start).id.to_string());
        log::error!("[{}] Total segments: {}", traversal_id, xpath.segments.len());
        log::error!("[{}] ─────────────────────────────────────────────────────────────────────────────", traversal_id);

        let segments = &xpath.segments;

        let mut current: Vec<Graph> = vec![Arc::clone(&start)];

        for (index, segment) in segments.iter().enumerate() {
            log::error!("[{}]", traversal_id);
            log::error!("[{}] ┌─ SEGMENT {}/{}", traversal_id, index + 1, segments.len());
            log::error!("[{}] │  Processing {} current graph(s)", traversal_id, current.len());

            current = current
                .iter()
                .map(|graph| {
                    Self::traverse_using_xpath_segment(
                        Arc::clone(&normalization_context),
                        Arc::clone(graph),
                        segment,
                    )
                })
                .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            log::error!("[{}] └─ After segment: {} graph(s) remaining", traversal_id, current.len());

            if current.is_empty() {
                if index == segments.len() - 1 {
                    log::error!("[{}] ╳ TRAVERSAL COMPLETE (all segments processed, no matches)", traversal_id);
                } else {
                    log::error!("[{}] ╳ TRAVERSAL STOPPED EARLY (no matches after segment {})", traversal_id, index);
                }

                log::error!("[{}] ─────────────────────────────────────────────────────────────────────────────", traversal_id);
                log::error!("╔════════════════════════════════════════════════════════════════════════════════╗");
                log::error!("║                  ✗ XPATH TRAVERSAL FAILED [ID: {}]                           ║", traversal_id);
                log::error!("╚════════════════════════════════════════════════════════════════════════════════╝");
                log::error!("");
                return Ok(None);
            }
        }

        let result_node = current.first().cloned();
        if let Some(ref node) = result_node {
            log::error!("[{}] ✓ SUCCESS - Selected node: {}", traversal_id, read_lock!(node).id.to_string());
        }
        log::error!("[{}] ─────────────────────────────────────────────────────────────────────────────", traversal_id);
        log::error!("╔════════════════════════════════════════════════════════════════════════════════╗");
        log::error!("║                  ✓ XPATH TRAVERSAL SUCCESS [ID: {}]                          ║", traversal_id);
        log::error!("╚════════════════════════════════════════════════════════════════════════════════╝");
        log::error!("");

        Ok(result_node)
    }

    pub fn to_xpath(&self, meta_context: &MetaContext) -> Result<XPath, Errors> {
        log::warn!("===== GENERATING XPATH FROM NODE =====");
        log::warn!("Source node ID: {}", self.id.to_string());

        let ancestors = {
            let mut ancestors: Vec<Graph> = Vec::new();
            let mut current_parents = self.parents.clone();

            while !current_parents.is_empty() {
                let parent = current_parents[0].clone();
                ancestors.push(parent.clone());
                current_parents = read_lock!(parent).parents.clone();
            }

            ancestors.reverse();
            log::info!("XPATH GENERATION - collected {} ancestors", ancestors.len());
            ancestors
        };

        let segments: Vec<XPathSegment> = ancestors
            .iter()
            .enumerate()
            .map(|(idx, graph)| {
                let lock = read_lock!(graph);
                let context = meta_context.contexts_lookup.get(&lock.id).unwrap();
                let document_node = read_lock!(context.document_node);

                let predicate = {
                    if lock.parents.len() > 0 {
                        let position = lock.index_in_parent_by_type(meta_context).unwrap();

                        if position > 0 {
                            Some(XPathPredicate::Position(position + 1))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                let element_name = document_node.get_element_name();
                log::debug!("XPATH GENERATION - ancestor segment {}: element='{}', position={:?}",
                           idx, element_name, predicate);

                XPathSegment {
                    axis: XPathAxis::Child,
                    node_test: element_name,
                    predicates: predicate.into_iter().collect(),
                }
            })
            .collect();

        let final_context = meta_context.contexts_lookup.get(&self.id).unwrap();

        let final_segment = {
            let position = {
                if self.parents.len() > 0 {
                    let pos = self.index_in_parent_by_type(meta_context).unwrap();
                    if pos > 0 {
                        Some(XPathPredicate::Position(pos + 1))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if final_context.data_node.fields.contains_key("text") {
                log::debug!("XPATH GENERATION - final segment: text() node");
                XPathSegment {
                    axis: XPathAxis::Child,
                    node_test: "text()".to_string(),
                    predicates: vec![],
                }
            } else {
                let document_node = read_lock!(final_context.document_node);
                let attributes: Vec<String> =
                    final_context.data_node.fields.keys().cloned().collect();
                let element_name = document_node.get_element_name();

                log::debug!("XPATH GENERATION - final segment: element='{}', position={:?}, attributes={:?}",
                           element_name, position, attributes);

                XPathSegment {
                    axis: XPathAxis::Child,
                    node_test: element_name,
                    predicates: position
                        .or(Some(XPathPredicate::AttributePresence(attributes)))
                        .into_iter()
                        .collect(),
                }
            }
        };

        let segments: Vec<XPathSegment> = segments
            .iter()
            .cloned()
            .chain(std::iter::once(final_segment))
            .collect();

        let segment_count = segments.len();
        let xpath = XPath { segments };

        log::info!("XPATH GENERATION - created xpath with {} total segments", segment_count);
        log::warn!("===== END XPATH GENERATION =====");

        Ok(xpath)
    }
}
