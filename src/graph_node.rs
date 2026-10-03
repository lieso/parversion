use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::basis_node::BasisNode;
use crate::data_node::DataNode;
use crate::prelude::*;
use crate::xpath::{XPath, XPathSegment, XPathPredicate, XPathAxis};

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

    pub fn document_path(&self) -> Vec<usize> {
        let mut path: Vec<usize> = Vec::new();
        let mut index = self.index_in_parent();
        let mut parent = self.parents.first().cloned();

        while let Some(p) = parent {
            path.push(index.unwrap_or(0));

            let lock = read_lock!(p);
            index = lock.index_in_parent();
            parent = lock.parents.first().cloned();
        }

        path.reverse();
        path
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
                log::debug!(
                    "XPATH GENERATION - ancestor segment {}: element='{}', position={:?}",
                    idx,
                    element_name,
                    predicate
                );

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

        log::info!(
            "XPATH GENERATION - created xpath with {} total segments",
            segment_count
        );
        log::warn!("===== END XPATH GENERATION =====");

        Ok(xpath)
    }
}
