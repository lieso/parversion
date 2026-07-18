use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::prelude::*;
use crate::xpath::{XPath, XPathAxis, XPathSegment, XPathPredicate};
use crate::basis_node::BasisNode;

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

    pub fn index_in_parent(&self) -> Option<usize> {
        self.parents.first().and_then(|parent| {
            read_lock!(parent)
                .children
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
            ancestors.push((parent.id.clone(), parent.hash.clone(), parent.index_in_parent()));
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
        normalization_context: Arc<RwLock<NormalizationContext>>
    ) -> Result<Option<Arc<BasisNode>>, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
        };

        let context_to_group = {
            let lock = read_lock!(normalization_context);
            lock.context_to_group.clone().ok_or(Errors::DeficientNormalizationContextError("'context_to_group' not provided in normalization context".to_string()))?
        };

        let context = meta_context.contexts_lookup
            .get(&self.id)
            .cloned()
            .unwrap();

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
        xpath_axis: &XPathAxis
    ) -> Result<Vec<Graph>, Errors> {
        let lock = read_lock!(graph);

        if lock.parents.len() > 1 {
            return Err(Errors::XPathTraverseError("Why are we traversing a graph using xpath if nodes have more than one parent?".to_string()));
        }

        match xpath_axis {
            XPathAxis::Child => {
                Ok(lock.children.clone())
            },
            XPathAxis::Parent => unimplemented!(),
            XPathAxis::Self_ => unimplemented!(),
            XPathAxis::Descendant => {
                let mut descendants = Vec::new();
                let mut queue = lock.children.clone();

                while !queue.is_empty() {
                    let node = queue.remove(0);
                    descendants.push(node.clone());
                    queue.extend(read_lock!(node).children.clone());
                }

                Ok(descendants)
            },
            XPathAxis::Ancestor => {
                let mut ancestors = Vec::new();
                let mut current_parents = lock.parents.clone();

                while !current_parents.is_empty() {
                    let parent = current_parents[0].clone();
                    ancestors.push(parent.clone());
                    let parent_read = read_lock!(parent);
                    current_parents = parent_read.parents.clone();
                }

                Ok(ancestors)
            },
            XPathAxis::FollowingSibling => {
                if let Some(parent) = lock.parents.first() {
                    if let Some(index_current) = read_lock!(parent).children.iter().position(|child| {
                        read_lock!(child).id == lock.id
                    }) {
                        let siblings: Vec<Graph> = read_lock!(parent).children[index_current + 1..].to_vec();
                        Ok(siblings)
                    } else {
                        Err(Errors::XPathTraverseError("Could not find index of current node as a child of parent".to_string()))
                    }
                } else {
                    Err(Errors::XPathTraverseError("Trying to visit following sibling on a root node".to_string()))
                }
            },
            XPathAxis::PrecedingSibling => {
                if let Some(parent) = lock.parents.first() {
                    if let Some(index_current) = read_lock!(parent).children.iter().position(|child| {
                        read_lock!(child).id == lock.id
                    }) {
                        let siblings: Vec<Graph> = read_lock!(parent).children[..index_current].to_vec();
                        Ok(siblings)
                    } else {
                        Err(Errors::XPathTraverseError("Could not find index of current node as a child of parent".to_string()))
                    }
                } else {
                    Err(Errors::XPathTraverseError("Trying to visit preceding sibling on a root node".to_string()))
                }
            },
            XPathAxis::Following => {
                let mut result = Vec::new();
                let mut current_id = lock.id.clone();
                let mut current_parents = lock.parents.clone();

                loop {
                    let Some(parent) = current_parents.first().cloned() else {
                        break;
                    };

                    let (next_id, next_parents, following_siblings) = {
                        let parent_lock = read_lock!(parent);
                        let Some(index) = parent_lock.children.iter().position(|child| {
                            read_lock!(child).id == current_id
                        }) else {
                            return Err(Errors::XPathTraverseError(
                                "Could not find index of current node as a child of parent".to_string()
                            ));
                        };
                        let following_siblings = parent_lock.children[index + 1..].to_vec();
                        (parent_lock.id.clone(), parent_lock.parents.clone(), following_siblings)
                    };

                    for sibling in following_siblings {
                        result.push(sibling.clone());
                        let mut queue = read_lock!(sibling).children.clone();
                        while !queue.is_empty() {
                            let desc = queue.remove(0);
                            result.push(desc.clone());
                            queue.extend(read_lock!(desc).children.clone());
                        }
                    }

                    current_id = next_id;
                    current_parents = next_parents;
                }

                Ok(result)
            },
        }
    }

    pub fn traverse_using_xpath_node_test(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graph: Graph,
        node_test: &String
    ) -> Result<Vec<Graph>, Errors> {
        if node_test == "node()" {
            panic!("Received node_test 'node()'");
        }

        if node_test == "text()" {
            panic!("Received node_test 'text()'");
        }

        if node_test == "comment()" {
            panic!("Received node_test 'comment()'");
        }

        if node_test == "*" {
            panic!("Received node_test '*'");
        }

        let contexts_lookup = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
        };
        let current_context = contexts_lookup.get(&read_lock!(graph).id).unwrap();
        let document_node = current_context.document_node.clone();
        let name = read_lock!(document_node).get_element_name();

        if node_test.trim() == name.trim() {
            Ok(vec![graph.clone()])
        } else {
            Ok(vec![])
        }
    }

    pub fn traverse_using_xpath_predicate(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graphs: Vec<Graph>,
        predicate: &XPathPredicate
    ) -> Result<Vec<Graph>, Errors> {
        match predicate {
            XPathPredicate::Position(index) => {
                // XPath positions are 1-indexed
                if *index < 1 || *index as usize > graphs.len() {
                    return Ok(vec![]);
                }

                let selected_graph = graphs.get(*index as usize - 1).cloned().unwrap();
                Ok(vec![selected_graph])
            }
            XPathPredicate::Attribute { name, value } => {
                let contexts_lookup = {
                    let lock = read_lock!(normalization_context);
                    lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
                };

                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        contexts_lookup
                            .get(&graph_id)
                            .and_then(|context| {
                                let _foo = read_lock!(&context.document_node)
                                    .get_attribute_value(name);

                                read_lock!(&context.document_node)
                                    .get_attribute_value(name)
                                    .map(|attr_value| attr_value.trim() == value.trim())
                            })
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                Ok(filtered)
            }
            XPathPredicate::AttributePresence(_) => {
                unimplemented!()
            }
        }
    }

    pub fn traverse_using_xpath_segment(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        graph: Graph,
        xpath_segment: &XPathSegment
    ) -> Result<Vec<Graph>, Errors> {
        let next_graphs: Vec<Graph> = Self::traverse_using_xpath_axis(
            Arc::clone(&normalization_context),
            Arc::clone(&graph),
            &xpath_segment.axis
        )?;

        let next_graphs: Vec<Graph> = next_graphs
            .iter()
            .map(|graph| {
                Self::traverse_using_xpath_node_test(
                    Arc::clone(&normalization_context),
                    Arc::clone(&graph),
                    &xpath_segment.node_test
                )
            })
            .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
            .into_iter()
            .flatten()
            .collect();

        if let Some(predicate) = &xpath_segment.predicate {
            let next_graphs = Self::traverse_using_xpath_predicate(
                Arc::clone(&normalization_context),
                next_graphs,
                &predicate
            )?;

            return Ok(next_graphs);
        }

        Ok(next_graphs)
    }

    pub fn traverse_using_xpath(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
        xpath: &XPath
    ) -> Result<Option<Graph>, Errors> {
        let segments = &xpath.segments;

        let mut current: Vec<Graph> = vec![Arc::clone(&start)];

        for (index, segment) in segments.iter().enumerate() {
            current = current
                .iter()
                .map(|graph| {
                    Self::traverse_using_xpath_segment(Arc::clone(&normalization_context), Arc::clone(graph), segment)
                })
                .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            if current.is_empty() {
                if index == segments.len() - 1 {
                    log::info!("All segments processed, but a graph was not found");
                } else {
                    log::info!("No matches after segment {}, stopping early", index);
                }

                return Ok(None);
            }
        }

        if current.len() > 1 {
            panic!("We found more than one graph matching provided xpath expression");
        }

        Ok(current.first().cloned())
    }

    pub fn to_xpath(
        &self,
        meta_context: &MetaContext
    ) -> Result<XPath, Errors> {
        let ancestors = {
            let mut ancestors: Vec<Graph> = Vec::new();
            let mut current_parents = self.parents.clone();

            while !current_parents.is_empty() {
                let parent = current_parents[0].clone();
                ancestors.push(parent.clone());
                current_parents = read_lock!(parent).parents.clone();
            }

            ancestors.reverse();
            ancestors
        };

        let segments: Vec<XPathSegment> = ancestors.iter().map(|graph| {
            let lock = read_lock!(graph);
            let context = meta_context.contexts_lookup.get(&lock.id).unwrap();
            let document_node = read_lock!(context.document_node);
            
            let predicate = {
                if lock.parents.len() > 0 {
                    let position = lock.index_in_parent().unwrap();

                    if position > 0 {
                        Some(XPathPredicate::Position(position + 1))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            XPathSegment {
                axis: XPathAxis::Child,
                node_test: document_node.get_element_name(),
                predicate,
            }
        }).collect();

        let final_context = meta_context.contexts_lookup.get(&self.id).unwrap();

        let final_segment = {
            if final_context.data_node.fields.contains_key("text") {
                XPathSegment {
                    axis: XPathAxis::Child,
                    node_test: "text()".to_string(),
                    predicate: None,
                }
            } else {
                let document_node = read_lock!(final_context.document_node);
                let attributes: Vec<String> = final_context.data_node.fields.keys().cloned().collect();

                XPathSegment {
                    axis: XPathAxis::Child,
                    node_test: document_node.get_element_name(),
                    predicate: Some(XPathPredicate::AttributePresence(attributes)),
                }
            }
        };

        let segments = segments
            .iter()
            .cloned()
            .chain(std::iter::once(final_segment))
            .collect();

        let xpath = XPath {
            segments,
        };

        Ok(xpath)
    }
}
