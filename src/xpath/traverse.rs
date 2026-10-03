use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::{Graph, GraphNode};
use super::{XPath, XPathAxis, XPathPredicate, XPathSegment, Value, Selection};

pub fn traverse_using_xpath_axis(
    _meta_context: Arc<RwLock<NormalizationContext>>,
    value: &Value,
    xpath_axis: &XPathAxis,
) -> Result<Vec<Value>, Errors> {
    let graph = value.graph.clone();
    let lock = read_lock!(graph);

    log::warn!("===== XPATH AXIS TRAVERSAL =====");
    log::warn!("XPATH AXIS: {:?}", xpath_axis);
    log::warn!("Current node ID: {}", lock.id.to_string());
    log::warn!(
        "Current node has {} children, {} parents",
        lock.children.len(),
        lock.parents.len()
    );

    if lock.parents.len() > 1 {
        log::error!(
            "ERROR: Node has multiple parents ({}) - xpath requires single parent",
            lock.parents.len()
        );
        return Err(Errors::XPathTraverseError(
            "Why are we traversing a graph using xpath if nodes have more than one parent?"
                .to_string(),
        ));
    }

    let result = match xpath_axis {
        XPathAxis::Child => {
            log::info!(
                "Applying XPATH AXIS::Child - returning {} children",
                lock.children.len()
            );
            for (idx, child) in lock.children.iter().enumerate() {
                log::debug!("  Child {}: {}", idx, read_lock!(child).id.to_string());
            }
            Ok(lock.children.clone())
        }
        XPathAxis::Parent => {
            log::info!(
                "Applying XPATH AXIS::Parent - returning {} parents",
                lock.parents.len()
            );
            for (idx, parent) in lock.parents.iter().enumerate() {
                log::debug!("  Parent {}: {}", idx, read_lock!(parent).id.to_string());
            }
            Ok(lock.parents.clone())
        }
        XPathAxis::Attribute => {
            log::info!("Applying XPATH AXIS::Attribute - staying on current node");
            Ok(vec![Arc::clone(&graph)])
        }
        XPathAxis::Self_ => {
            log::info!("Applying XPATH AXIS::Self_ - returning current node");
            Ok(vec![graph.clone()])
        }
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
                log::debug!(
                    "  [Descendant depth {}] Added node {}, has {} children",
                    depth,
                    node_lock.id.to_string(),
                    node_lock.children.len()
                );
                queue.extend(node_lock.children.clone());
                depth += 1;
            }

            log::info!(
                "XPATH AXIS::Descendant - found {} total descendants",
                descendants.len()
            );
            Ok(descendants)
        }
        XPathAxis::Ancestor => {
            log::info!("Applying XPATH AXIS::Ancestor - traversing all ancestors");
            let mut ancestors = Vec::new();
            let mut current_parents = lock.parents.clone();
            let mut depth = 0;

            while !current_parents.is_empty() {
                let parent = current_parents[0].clone();
                let parent_lock = read_lock!(parent);
                ancestors.push(parent.clone());
                log::debug!(
                    "  [Ancestor depth {}] Added ancestor {}, has {} parents",
                    depth,
                    parent_lock.id.to_string(),
                    parent_lock.parents.len()
                );
                current_parents = parent_lock.parents.clone();
                depth += 1;
            }

            log::info!(
                "XPATH AXIS::Ancestor - found {} total ancestors",
                ancestors.len()
            );
            Ok(ancestors)
        }
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
        }
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
        }
        XPathAxis::Following => {
            log::info!("Applying XPATH AXIS::Following - collecting all following nodes");
            let mut result = Vec::new();
            let mut current_id = lock.id.clone();
            let mut current_parents = lock.parents.clone();
            let mut iteration = 0;

            loop {
                let Some(parent) = current_parents.first().cloned() else {
                    log::debug!(
                        "  [Following iteration {}] Reached root (no parent)",
                        iteration
                    );
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
                    log::trace!(
                        "    [Following] Added sibling {}",
                        sibling_lock.id.to_string()
                    );
                    let mut queue = sibling_lock.children.clone();
                    while !queue.is_empty() {
                        let desc = queue.remove(0);
                        let desc_lock = read_lock!(desc);
                        result.push(desc.clone());
                        log::trace!(
                            "    [Following] Added descendant {}",
                            desc_lock.id.to_string()
                        );
                        queue.extend(desc_lock.children.clone());
                    }
                }

                current_id = next_id;
                current_parents = next_parents;
                iteration += 1;
            }

            log::info!(
                "XPATH AXIS::Following - collected {} total following nodes",
                result.len()
            );
            Ok(result)
        }
        XPathAxis::Preceding => {
            log::info!("Applying XPATH AXIS::Preceding - collecting all preceding nodes");
            let mut result = Vec::new();
            let mut current_id = lock.id.clone();
            let mut current_parents = lock.parents.clone();
            let mut iteration = 0;

            loop {
                let Some(parent) = current_parents.first().cloned() else {
                    log::debug!(
                        "  [Preceding iteration {}] Reached root (no parent)",
                        iteration
                    );
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
                    log::trace!(
                        "    [Preceding] Added sibling {}",
                        sibling_lock.id.to_string()
                    );
                    let mut queue = sibling_lock.children.clone();
                    while !queue.is_empty() {
                        let desc = queue.remove(0);
                        let desc_lock = read_lock!(desc);
                        result.push(desc.clone());
                        log::trace!(
                            "    [Preceding] Added descendant {}",
                            desc_lock.id.to_string()
                        );
                        queue.extend(desc_lock.children.clone());
                    }
                }

                current_id = next_id;
                current_parents = next_parents;
                iteration += 1;
            }

            log::info!(
                "XPATH AXIS::Preceding - collected {} total preceding nodes",
                result.len()
            );
            Ok(result)
        }
    };

    log::warn!("===== END XPATH AXIS TRAVERSAL =====");
    result.map(|graphs| graphs.into_iter().map(Value::from_graph).collect())
}

pub fn traverse_using_xpath_node_test(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    value: &Value,
    node_test: &String,
) -> Result<Vec<Value>, Errors> {
    let graph = value.graph.clone();
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
        log::info!(
            "XPATH NODE TEST: MATCH - node_test matches element name, returning current node"
        );
        Ok(vec![value.clone()])
    } else {
        log::info!(
            "XPATH NODE TEST: NO MATCH - node_test '{}' != element name '{}'",
            node_test.trim(),
            name.trim()
        );
        Ok(vec![])
    };

    log::warn!("===== END XPATH NODE TEST =====");
    result
}

pub fn traverse_using_xpath_predicate(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    values: Vec<Value>,
    predicate: &XPathPredicate,
) -> Result<Vec<Value>, Errors> {
    log::warn!("===== XPATH PREDICATE =====");
    log::warn!("PREDICATE: {:?}", predicate);
    log::warn!("Input values count: {}", values.len());

    let result = match predicate {
        XPathPredicate::Position(index) => {
            log::info!(
                "XPATH PREDICATE::Position - filtering for position {}",
                index
            );
            // XPath positions are 1-indexed
            if *index < 1 || *index as usize > values.len() {
                log::info!("XPATH PREDICATE::Position {} - OUT OF BOUNDS (values.len={}), returning empty", index, values.len());
                return Ok(vec![]);
            }

            let selected = values.get(*index as usize - 1).cloned().unwrap();
            log::info!(
                "XPATH PREDICATE::Position {} - MATCH found, selecting node {}",
                index,
                read_lock!(selected.graph).id.to_string()
            );
            Ok(vec![selected])
        }
        XPathPredicate::Last => {
            log::info!(
                "XPATH PREDICATE::Last - selecting last value from {} values",
                values.len()
            );
            let result = values.last().cloned().into_iter().collect();
            if let Some(last_value) = values.last() {
                log::info!(
                    "XPATH PREDICATE::Last - selected node {}",
                    read_lock!(last_value.graph).id.to_string()
                );
            }
            Ok(result)
        }
        XPathPredicate::Not(inner) => {
            log::info!("XPATH PREDICATE::Not - applying inner predicate to filter");
            let mut filtered = Vec::new();
            let mut matched_count = 0;
            for value in values {
                let matched = traverse_using_xpath_predicate(
                    Arc::clone(&normalization_context),
                    vec![value.clone()],
                    inner,
                )?;
                if matched.is_empty() {
                    filtered.push(value);
                } else {
                    matched_count += 1;
                }
            }
            log::info!(
                "XPATH PREDICATE::Not - filtered {} matched, kept {} unmatched",
                matched_count,
                filtered.len()
            );
            Ok(filtered)
        }
        XPathPredicate::ContainsNormalized { value } => {
            log::info!("XPATH PREDICATE::ContainsNormalized - filtering for normalized text containing '{}'", value);
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
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
            log::info!(
                "XPATH PREDICATE::ContainsNormalized - matched {} out of {} values",
                matched_count,
                values.len()
            );
            Ok(filtered)
        }
        XPathPredicate::Contains { name, value } => {
            log::info!(
                "XPATH PREDICATE::Contains - filtering for attribute '{}' containing '{}'",
                name,
                value
            );
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    let matches = contexts_lookup
                        .get(&graph_id)
                        .and_then(|context| {
                            let doc_node = read_lock!(&context.document_node);
                            log::trace!(
                                "XPATH PREDICATE::Contains - checking node {} ({})",
                                graph_id.to_string(),
                                doc_node.to_string()
                            );
                            doc_node
                                .get_attribute_value(name)
                                .map(|attr_value| attr_value.trim().contains(value.trim()))
                        })
                        .unwrap_or(false);
                    if matches {
                        if let Some(context) = contexts_lookup.get(&graph_id) {
                            let doc_node = read_lock!(&context.document_node);
                            log::debug!(
                                "XPATH PREDICATE::Contains - MATCH on node {} ({})",
                                graph_id.to_string(),
                                doc_node.to_string()
                            );
                        }
                        matched_count += 1;
                    }
                    matches
                })
                .cloned()
                .collect();

            log::info!(
                "XPATH PREDICATE::Contains - matched {} out of {} values",
                matched_count,
                values.len()
            );

            Ok(filtered)
        }
        XPathPredicate::ContainsToken { name, value } => {
            log::info!(
                "XPATH PREDICATE::ContainsToken - filtering for attribute '{}' containing token '{}'",
                name,
                value
            );
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    contexts_lookup
                        .get(&graph_id)
                        .and_then(|context| {
                            read_lock!(&context.document_node).get_attribute_value(name)
                        })
                    .map(|attr_value| attr_value.split_whitespace().any(|word| word == value))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            log::info!(
                "XPATH PREDICATE::ContainsToken - matched {} out of {} values",
                filtered.len(),
                values.len()
            );
            Ok(filtered)
        }
        XPathPredicate::Attribute { name, value } => {
            log::info!(
                "XPATH PREDICATE::Attribute - filtering for attribute match: {}='{}'",
                name,
                value
            );
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    let matches = contexts_lookup
                        .get(&graph_id)
                        .and_then(|context| {
                            let doc_node = read_lock!(&context.document_node);
                            log::trace!(
                                "XPATH PREDICATE::Attribute - checking node {} ({})",
                                graph_id.to_string(),
                                doc_node.to_string()
                            );
                            doc_node.get_attribute_value(name).map(|attr_value| {
                                let attr_value = attr_value.trim();
                                let value = value.trim();

                                // Preserve original exact-match semantics unconditionally.
                                if attr_value == value {
                                    return true;
                                }

                                // Additionally treat single-token values as matching
                                // any whitespace-separated token in the attribute
                                // (e.g. @class='commtext' matching class="commtext c00").
                                // Skip this for multi-word values, since a multi-word
                                // value can never equal a single token anyway.
                                if !value.is_empty() && !value.contains(char::is_whitespace) {
                                    attr_value.split_whitespace().any(|token| token == value)
                                } else {
                                    false
                                }
                            })
                        })
                        .unwrap_or(false);
                    if matches {
                        if let Some(context) = contexts_lookup.get(&graph_id) {
                            let doc_node = read_lock!(&context.document_node);
                            log::debug!(
                                "XPATH PREDICATE::Attribute - MATCH on node {} ({}, {}='{}')",
                                graph_id.to_string(),
                                doc_node.get_element_name(),
                                name,
                                value
                            );
                        }
                        matched_count += 1;
                    }
                    matches
                })
                .cloned()
                .collect();

            log::info!(
                "XPATH PREDICATE::Attribute - matched {} out of {} values",
                matched_count,
                values.len()
            );
            Ok(filtered)
        }
        XPathPredicate::AttributePresence(names) => {
            log::info!("XPATH PREDICATE::AttributePresence - filtering for presence of {} attributes: {:?}", names.len(), names);
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    let matches = contexts_lookup
                        .get(&graph_id)
                        .map(|context| {
                            let doc_node = read_lock!(&context.document_node);
                            log::trace!(
                                "XPATH PREDICATE::AttributePresence - checking node {} ({})",
                                graph_id.to_string(),
                                doc_node.to_string()
                            );
                            names
                                .iter()
                                .all(|name| doc_node.get_attribute_value(name).is_some())
                        })
                        .unwrap_or(false);
                    if matches {
                        if let Some(context) = contexts_lookup.get(&graph_id) {
                            let doc_node = read_lock!(&context.document_node);
                            log::debug!(
                                "XPATH PREDICATE::AttributePresence - MATCH on node {} ({})",
                                graph_id.to_string(),
                                doc_node.get_element_name()
                            );
                        }
                        matched_count += 1;
                    }
                    matches
                })
                .cloned()
                .collect();

            log::info!(
                "XPATH PREDICATE::AttributePresence - matched {} out of {} values",
                matched_count,
                values.len()
            );
            Ok(filtered)
        }
        XPathPredicate::StartsWith { name, value } => {
            log::info!(
                "XPATH PREDICATE::StartsWith - filtering for attribute '{}' starting with '{}'",
                name,
                value
            );
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };

            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .iter()
                .filter(|v| {
                    let graph = v.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    let matches = contexts_lookup
                        .get(&graph_id)
                        .and_then(|context| {
                            let doc_node = read_lock!(&context.document_node);
                            log::trace!(
                                "XPATH PREDICATE::StartsWith - checking node {} ({})",
                                graph_id.to_string(),
                                doc_node.to_string()
                            );
                            doc_node
                                .get_attribute_value(name)
                                .map(|attr_value| attr_value.trim().starts_with(value.trim()))
                        })
                        .unwrap_or(false);
                    if matches {
                        if let Some(context) = contexts_lookup.get(&graph_id) {
                            let doc_node = read_lock!(&context.document_node);
                            log::debug!(
                                "XPATH PREDICATE::StartsWith - MATCH on node {} ({})",
                                graph_id.to_string(),
                                doc_node.get_element_name()
                            );
                        }
                        matched_count += 1;
                    }
                    matches
                })
                .cloned()
                .collect();

            log::info!(
                "XPATH PREDICATE::StartsWith - matched {} out of {} values",
                matched_count,
                values.len()
            );

            Ok(filtered)
        }
        XPathPredicate::Path(path) => {
            log::info!("XPATH PREDICATE::Path - filtering based on path traversal");
            let mut matched_count = 0;
            let filtered: Vec<Value> = values
                .into_iter()
                .filter(|value| {
                    let graph = value.graph.clone();
                    let graph_id = read_lock!(graph).id.clone();
                    let path_match = path
                        .traverse(Arc::clone(&normalization_context), Arc::clone(&graph))
                        .map(|found| !found.is_empty())
                        .unwrap_or(false);

                    if path_match {
                        log::debug!(
                            "XPATH PREDICATE::Path - MATCH on node {}",
                            graph_id.to_string()
                        );
                        matched_count += 1;
                    }
                    path_match
                })
                .collect();

            log::info!(
                "XPATH PREDICATE::Path - matched {} values via path traversal",
                matched_count
            );
            Ok(filtered)
        }
        XPathPredicate::And(predicates) => {
            log::info!(
                "XPATH PREDICATE::And - applying {} predicates sequentially",
                predicates.len()
            );
            predicates.iter().try_fold(values, |acc, predicate| {
                log::debug!(
                    "XPATH PREDICATE::And - applying predicate to {} values",
                    acc.len()
                );
                traverse_using_xpath_predicate(
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
    value: &Value,
    xpath_segment: &XPathSegment,
) -> Result<Vec<Value>, Errors> {
    let graph = value.graph.clone();
    let graph_id = read_lock!(graph).id.clone();

    log::warn!("===== XPATH SEGMENT =====");
    log::warn!(
        "SEGMENT - axis: {:?}, node_test: '{}', predicates: {}",
        xpath_segment.axis,
        xpath_segment.node_test,
        xpath_segment.predicates.len()
    );
    log::warn!("Starting node ID: {}", graph_id.to_string());

    let contexts_lookup = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
    };
    if let Some(context) = contexts_lookup.get(&graph_id) {
        let doc_node = read_lock!(context.document_node);
        log::warn!("  DocumentNode: {}", doc_node.to_string());
    }

    let mut next_values: Vec<Value> = traverse_using_xpath_axis(
        Arc::clone(&normalization_context),
        value,
        &xpath_segment.axis,
    )?;

    log::info!(
        "XPATH SEGMENT - after axis '{}', have {} values",
        format!("{:?}", xpath_segment.axis),
        next_values.len()
    );

    let mut next_values: Vec<Value> = if matches!(
        xpath_segment.axis,
        XPathAxis::Self_ | XPathAxis::Parent | XPathAxis::Attribute
    ) {
        log::info!("XPATH SEGMENT - skipping node_test for Self_/Parent axis");
        next_values
    } else {
        log::info!(
            "XPATH SEGMENT - applying node_test '{}' to {} graphs",
            xpath_segment.node_test,
            next_values.len()
        );
        let tested: Vec<Vec<Value>> = next_values
            .iter()
            .map(|v| {
                traverse_using_xpath_node_test(
                    Arc::clone(&normalization_context),
                    v,
                    &xpath_segment.node_test,
                )
            })
            .collect::<Result<Vec<Vec<Value>>, Errors>>()?;

        let flattened: Vec<Value> = tested.into_iter().flatten().collect();
        log::info!(
            "XPATH SEGMENT - after node_test, have {} graphs",
            flattened.len()
        );
        flattened
    };

    log::info!(
        "XPATH SEGMENT - applying {} predicates",
        xpath_segment.predicates.len()
    );
    let mut predicate_count = 0;
    let result =
        xpath_segment
            .predicates
            .iter()
            .try_fold(next_values, |values, predicate| {
                predicate_count += 1;
                log::debug!(
                    "XPATH SEGMENT - predicate {}/{}: {} values before",
                    predicate_count,
                    xpath_segment.predicates.len(),
                    values.len()
                );
                let result = traverse_using_xpath_predicate(
                    Arc::clone(&normalization_context),
                    values,
                    predicate,
                );
                if let Ok(ref filtered) = result {
                    log::debug!(
                        "XPATH SEGMENT - predicate {}/{}: {} values after",
                        predicate_count,
                        xpath_segment.predicates.len(),
                        filtered.len()
                    );
                }
                result
            })?;

    log::info!("XPATH SEGMENT - final result: {} values", result.len());
    log::warn!("===== END XPATH SEGMENT =====");

    Ok(result)
}
