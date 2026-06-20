use serde_json::{json, Value, Map};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::fmt::Write;

use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode, GraphNodeID};
use crate::json_node::JsonNode;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::basis_group::BasisGroup;
use crate::document::Document;
use crate::document_format::DocumentFormat;

pub type ContextID = ID;

#[derive(Clone, Debug)]
pub struct Context {
    pub id: ContextID,
    pub lineage: Lineage,
    pub acyclic_lineage: Lineage,
    pub document_node: Arc<RwLock<DocumentNode>>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
    pub network_name: String,
}

impl Context {
    pub fn generate_context_string(&self, meta_context: &MetaContext) -> Result<String, Errors> {
        let spatial_context: String = self.generate_spatial_context(meta_context)?;
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
            return Ok(context_string);
        }

        let context_strings: Vec<String> = self.data_node.fields.keys().map(|key| {
            format!("{} -> {}", context_string, key)
        }).collect();

        Ok(context_strings.join("\n"))
    }

    fn generate_spatial_context(&self, meta_context: &MetaContext) -> Result<String, Errors> {
        let mut neighbourhood = HashSet::new();

        traverse_structural_envelope(
            Arc::clone(&self.graph_node),
            &mut neighbourhood
        );

        let document_type = read_lock!(self.document_node).get_document_type();

        let partial_document = Document::from_meta_context(
            meta_context,
            &DocumentFormat {
                format_type: document_type,
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
}





impl Context {
    pub fn get_indexed_lineage(&self, depth: usize) -> Option<Lineage> {
        let graph_node = read_lock!(self.graph_node);
        graph_node.get_indexed_lineage_at_depth(depth)
    }

    pub fn generate_json_snippet(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>
    ) -> Result<Map<String, Value>, Errors> {



        let mut result: Map<String, Value> = Map::new();


        fn recurse(
            normalization_context: Arc<RwLock<NormalizationContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>
        ) {



            let contexts = {
                let lock = read_lock!(normalization_context);
                lock.meta_context.as_ref().unwrap().contexts_lookup.clone()
            };
            let context_to_group = {
                let lock = read_lock!(normalization_context);
                lock.context_to_group.clone().unwrap()
            };





            let mut subgraph_counter: HashMap<String, u32> = HashMap::new();
            for child in &read_lock!(graph_node).children {
                let subgraph_hash: String = read_lock!(child).subgraph_hash.clone().to_string().unwrap();

                let count = *subgraph_counter.entry(subgraph_hash.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);

                if count <= 3 {
                    let mut inner_result: Map<String, Value> = Map::new();

                    recurse(
                        Arc::clone(&normalization_context),
                        Arc::clone(&child),
                        &mut inner_result,
                    );

                    let inner_result_value = Value::Object(inner_result.clone());

                    if let Some(existing_object) = result.get_mut(&subgraph_hash) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }
                    } else {
                        if inner_result.len() > 0 {
                            result.insert(subgraph_hash.to_string(), inner_result_value);
                        }
                    }
                }
            }





            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let data_node = &context.data_node;
            let maybe_basis_group: Option<Arc<BasisGroup>> = context_to_group.get(&context.id).cloned();

            let basis_lineage: Option<Lineage> = {
                if let Some(basis_group) = maybe_basis_group {
                    Some(basis_group.get_basis_lineage())
                } else {
                    None
                }
            };

            if let Some(basis_lineage) = basis_lineage {
                let basis_node = {
                    let lock = read_lock!(normalization_context);
                    lock.get_basis_node_by_lineage(&basis_lineage)
                        .expect("Could not get basis node by lineage")
                        .unwrap()
                };
                let json_nodes: Vec<JsonNode> = basis_node
                    .transformations
                    .clone()
                    .into_iter()
                    .flat_map(|transformation| {
                        transformation
                            .transform(Arc::clone(&data_node))
                            .expect("Could not transform data node field")
                            .to_json_nodes()
                    })
                    .collect();

                let _json_data: Map<String, Value> = Map::new();

                for json_node in json_nodes.into_iter() {
                    let json = json_node.json;
                    let value = json!(json.value.trim().to_string());
                    result.insert(json.key, value);
                }

                //if json_data.len() > 0 {
                //    result.insert("_json".to_string(), Value::Object(json_data));
                //}

            }


        }

        recurse(
            Arc::clone(&normalization_context),
            Arc::clone(&self.graph_node),
            &mut result,
        );

        Ok(result)

        //Ok(serde_json::to_string_pretty(&result).expect("Could not make a JSON string"))
    }
    
    pub fn generate_snippet(&self, normalization_context: Arc<RwLock<NormalizationContext>>) -> String {
        let mut neighbour_ids = HashSet::new();
        let graph_node = self.graph_node.clone();

        Self::traverse_for_neighbours(Arc::clone(&graph_node), &mut neighbour_ids, &20);

        let mut snippet = String::new();
        let graph_root = {
            let lock = read_lock!(normalization_context);
            lock.meta_context.as_ref().unwrap().graph_root.clone()
        };

        Self::traverse_for_snippet(
            Arc::clone(&normalization_context),
            Arc::clone(&graph_root),
            &neighbour_ids,
            &read_lock!(graph_node).id,
            &mut snippet,
        );

        snippet
    }

    fn traverse_for_snippet(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        current_node: Arc<RwLock<GraphNode>>,
        neighbour_ids: &HashSet<GraphNodeID>,
        target_id: &GraphNodeID,
        snippet: &mut String,
    ) {
        let normalization_context_lock = read_lock!(normalization_context);
        let lock = read_lock!(current_node);
        let current_id = lock.id.clone();
        let contexts_lookup = &normalization_context_lock.meta_context.as_ref().unwrap().contexts_lookup;
        let current_context = contexts_lookup.get(&current_id).unwrap();
        let document_node = current_context.document_node.clone();

        let should_render = if current_id == *target_id {
            let (mut a, _b) = read_lock!(document_node).to_string_components();

            a = Self::mark_text(&a);
            snippet.push_str(&a);

            true
        } else if neighbour_ids.contains(&current_id) {
            let (a, _b) = read_lock!(document_node).to_string_components();

            snippet.push_str(&a);

            true
        } else {
            false
        };

        for child in &lock.children {
            Self::traverse_for_snippet(
                Arc::clone(&normalization_context),
                Arc::clone(child),
                neighbour_ids,
                target_id,
                snippet,
            );
        }

        if should_render {
            let (_, b) = read_lock!(document_node).to_string_components();

            snippet.push_str(b.as_deref().unwrap_or(""));
        }
    }

    fn traverse_for_neighbours(
        start_node: Arc<RwLock<GraphNode>>,
        visited: &mut HashSet<GraphNodeID>,
        max_neighbours: &usize
    ) {
        let mut queue: VecDeque<Arc<RwLock<GraphNode>>> = VecDeque::new();
        queue.push_back(Arc::clone(&start_node));

        while let Some(node) = queue.pop_front() {
            let lock = read_lock!(node);
            let graph_node_id = lock.id.clone();

            if visited.contains(&graph_node_id) {
                continue;
            }

            visited.insert(graph_node_id.clone());

            if visited.len() > *max_neighbours {
                return;
            }

            for child in lock.children.iter() {
                queue.push_back(Arc::clone(child));
            }

            for parent in lock.parents.iter() {
                queue.push_back(Arc::clone(parent));
            }
        }
    }

    fn mark_text(text: &str) -> String {
        let marker_prefix = "<!-- Target node: Start -->";
        let marker_suffix = "<!-- Target node: End -->";

        format!("{}{}{}", marker_prefix, text, marker_suffix)
    }
}











fn traverse_structural_envelope(
    target_node: Graph,
    neighbourhood: &mut HashSet<GraphNodeID>,
) {
    // ******************************************
    let max_neighbours = 30;
    let max_children = 5;
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

        for child in children_to_enqueue.iter() {
            queue.push_back(Arc::clone(child));
        }

        for parent in lock.parents.iter() {
            queue.push_back(Arc::clone(parent));
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
