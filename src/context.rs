use serde_json::{json, Value, Map};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode, GraphNodeID};
use crate::json_node::JsonNode;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::basis_group::BasisGroup;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::basis_node::BasisNode;

pub type ContextID = ID;

#[derive(Clone, Debug)]
pub struct Context {
    pub id: ContextID,
    pub lineage: Lineage,
    pub acyclic_lineage: Lineage,
    pub indexed_lineages: Arc<RwLock<HashMap<usize, Lineage>>>,
    pub document_node: Arc<RwLock<DocumentNode>>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
    pub network_name: String,
}

impl Context {
    pub fn generate_context_string_node_relationship(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        basis_node: Arc<BasisNode>
    ) -> Result<String, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string())
                })?
                .clone()
        };

        let spatial_context: String = self.generate_spatial_context(&meta_context, Vec::new())?;
        let positional_context: String = self.generate_positional_context(&meta_context)?;

        let mut transformed_context = String::new();
        for transformation in &basis_node.transformations {
            let transformed = transformation.transform(self.data_node.clone())?;

            for value in transformed.fields.get(&transformation.image) {
                transformed_context.push_str(&format!("{} => {}", transformation.image, value));
            }
        }

        let result = format!(r##"
[SPATIAL CONTEXT]
{}

[POSITIONAL CONTEXT]
{}

[TRANSFORMED FIELDS]
{}
"##, spatial_context, positional_context, transformed_context);

        Ok(result)
    }

    pub fn generate_context_string_basis_group(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>
    ) -> Result<String, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string())
                })?
                .clone()
        };

        let spatial_context: String = self.generate_spatial_context(&meta_context, Vec::new())?;
        let positional_context: String = self.generate_positional_context(&meta_context)?;


        let fields_context: String = self.data_node
            .fields
            .iter()
            .fold(String::new(), |acc, (field, value)| {
                format!("{}\nFIELD: {}, VALUE: {}", acc, field, value)
            });


        let result = format!(r##"
[SPATIAL CONTEXT]
{}

[POSITIONAL CONTEXT]
{}

[EXTRACTED FIELDS]{}
"##, spatial_context, positional_context, fields_context);

        Ok(result)
    }

    pub fn generate_context_string_basis_network(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        relevant_contexts: Vec<Arc<Context>>,
    ) -> Result<String, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string())
                })?
                .clone()
        };

        let mut context_string = self.generate_context_string(&meta_context, relevant_contexts.clone())?;

        if read_lock!(normalization_context).basis_nodes.is_some() {
            let basis_nodes_context_string = self.generate_basis_nodes_context(
                Arc::clone(&normalization_context),
                relevant_contexts.clone()
            )?;

            context_string.push_str(&basis_nodes_context_string);
        }

        Ok(context_string)
    }

    pub fn generate_context_string(&self, meta_context: &MetaContext, relevant_contexts: Vec<Arc<Context>>) -> Result<String, Errors> {
        let spatial_context: String = self.generate_spatial_context(meta_context, relevant_contexts)?;
        let positional_context: String = self.generate_positional_context(meta_context)?;

        let result = format!(r##"
[SPATIAL CONTEXT]
{}

[POSITIONAL CONTEXT]
{}
"##, spatial_context, positional_context);

        Ok(result)
    }

    fn generate_positional_context(&self, meta_context: &MetaContext) -> Result<String, Errors> {
        match meta_context.document_type {
            DocumentType::Json => self.generate_positional_context_json(meta_context),
            DocumentType::Html => self.generate_positional_context_html(meta_context),
            _ => unimplemented!()
        }
    }

    fn generate_positional_context_html(&self, meta_context: &MetaContext) -> Result<String, Errors> {
        let xpath = {
            let lock = read_lock!(self.graph_node);
            lock.to_xpath(meta_context)?
        };

        Ok(xpath.to_string())
    }

    fn generate_positional_context_json(&self, meta_context: &MetaContext) -> Result<String, Errors> {
        let root_to_target = get_path_to_target(Arc::clone(&self.graph_node));
        let context_string = root_to_target.iter().fold(String::new(), |acc, graph| {
            let current_context = meta_context.contexts_lookup.get(&read_lock!(graph).id).unwrap();

            if current_context.network_name.is_empty() {
                acc
            } else {
                if acc.is_empty() {
                    format!("{}", current_context.network_name)
                } else {
                    format!("{} -> {}", acc, current_context.network_name)
                }
            }
        });
        if self.data_node.fields.is_empty() {
            if self.network_name.is_empty() {
                panic!("network name is empty");
            } else {
                if context_string.is_empty() {
                    let positional_context = format!("{}", self.network_name);
                    return Ok(positional_context);
                } else {
                    let positional_context = format!("{} -> {}", context_string, self.network_name);
                    return Ok(positional_context);
                }
            }
        }

        let context_strings: Vec<String> = self.data_node.fields.keys().map(|key| {
            format!("{} -> {}", context_string, key)
        }).collect();

        Ok(context_strings.join("\n"))
    }

    fn generate_spatial_context(&self, meta_context: &MetaContext, relevant_contexts: Vec<Arc<Context>>) -> Result<String, Errors> {
        let mut neighbourhood = HashSet::new();

        self.traverse_structural_envelope(
            &mut neighbourhood
        );

        for context in relevant_contexts {
            context.traverse_structural_envelope(
                &mut neighbourhood
            );
        }

        let partial_document = Document::from_meta_context(
            meta_context,
            &DocumentFormat {
                format_type: meta_context.document_type.clone(),
                encoding: Some(String::from("UTF-8")),
                indent: None,
                line_ending: None,
                headers: None,
                wrap_text: None,
                exclude_nulls: None,
                custom_delimiter: None,
            },
            Some(&neighbourhood)
        )?;

        Ok(partial_document.to_string())
    }

    fn generate_basis_nodes_context(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        relevant_contexts: Vec<Arc<Context>>
    ) -> Result<String, Errors> {

        let mut result: String = String::new();

        fn recurse(
            normalization_context: Arc<RwLock<NormalizationContext>>,
            graph: Graph,
            result: &mut String,
            relevant_contexts: &Vec<Arc<Context>>
        ) -> Result<(), Errors> {
            let lock = read_lock!(graph);

            let meta_context = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
            };

            let context = meta_context.contexts_lookup
                .get(&lock.id)
                .cloned()
                .unwrap();

            if relevant_contexts.iter().any(|c| c.id == context.id) {
                if let Some(basis_node) = lock.resolve_basis_node(Arc::clone(&normalization_context))? {
                    for transformation in &basis_node.transformations {
                        let transformed = transformation.transform(context.data_node.clone())?;

                        for value in transformed.fields.get(&transformation.image) {
                            result.push_str(&format!("{} => {} (value = {})\n", transformation.field, transformation.image, value));
                        }
                    }
                }
            }
            
            for child in &lock.children {
                recurse(
                    Arc::clone(&normalization_context),
                    Arc::clone(&child),
                    result,
                    relevant_contexts
                );
            }

            Ok(())
        }

        recurse(
            Arc::clone(&normalization_context),
            self.graph_node.clone(),
            &mut result,
            &relevant_contexts
        )?;


        let result = format!(r##"
[TRANSFORMED NODES]
{}
"##, result);

        Ok(result)
    }
}

impl Context {
    pub fn get_indexed_lineage(&self, depth: usize) -> Option<Lineage> {
        {
            let cache = read_lock!(self.indexed_lineages);
            if let Some(lineage) = cache.get(&depth) {
                return Some(lineage.clone());
            }
        }

        let graph_node = read_lock!(self.graph_node);
        if let Some(lineage) = graph_node.get_indexed_lineage_at_depth(depth) {
            let mut cache = write_lock!(self.indexed_lineages);
            cache.insert(depth, lineage.clone());
            Some(lineage)
        } else {
            None
        }
    }

    fn traverse_structural_envelope(
        &self,
        neighbourhood: &mut HashSet<GraphNodeID>,
    ) {
        let target_node = &self.graph_node;

        // ******************************************
        let max_neighbours = 50;
        let max_children = 6;
        // ******************************************
        
        let mut queue: VecDeque<Graph> = VecDeque::new();
        queue.push_back(Arc::clone(&target_node));

        while let Some(node) = queue.pop_front() {
            let lock = read_lock!(node);

            if neighbourhood.contains(&lock.id) {
                continue;
            }

            neighbourhood.insert(lock.id.clone());

            if neighbourhood.len() > max_neighbours {
                return;
            }

            // Center the children around the target node,
            // only if one of these children is the target_node
            let children = lock.children.clone();

            let children_to_enqueue = if children.iter().any(|child| {
                read_lock!(child).id == read_lock!(target_node).id
            }) {
                let target_node_position = children.iter().position(|child| {
                    read_lock!(child).id == read_lock!(target_node).id
                }).unwrap();

                let half = max_children / 2;
                let start = target_node_position.saturating_sub(half);
                let end = (start + max_children).min(children.len());

                &children[start..end]
            } else {
                &children[..max_children.min(children.len())]
            };

            for child in &lock.children {
                queue.push_back(Arc::clone(&child));
            }

            for parent in lock.parents.iter() {
                queue.push_back(Arc::clone(parent));
            }
        }
    }
}


fn get_path_to_target(target_node: Graph) -> Vec<Graph> {
    let mut ancestors: Vec<Graph> = Vec::new();
    let mut current_parents = read_lock!(target_node).parents.clone();

    while !current_parents.is_empty() {
        let parent = current_parents[0].clone();
        ancestors.push(parent.clone());
        current_parents = read_lock!(parent).parents.clone();
    }

    ancestors.reverse();
    ancestors
}
