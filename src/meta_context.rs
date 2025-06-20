use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap, VecDeque};
use serde_json::{json, Value};

use crate::prelude::*;
use crate::graph_node::{Graph};
use crate::context::{Context};
use crate::basis_graph::BasisGraph;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::profile::Profile;
use crate::transformation::SchemaTransformation;
use crate::document::{Document, DocumentType, DocumentMetadata};
use crate::schema_node::SchemaNode;
use crate::schema::Schema;
use crate::json_node::JsonNode;
use crate::document_format::{DocumentFormat};
use crate::basis_network::{NetworkRelationship};

pub struct MetaContext {
    pub contexts: Option<HashMap<ID, Arc<Context>>>,
    pub graph_root: Option<Graph>,
    pub basis_nodes: Option<HashMap<ID, Arc<BasisNode>>>,
    pub basis_networks: Option<HashMap<ID, Arc<BasisNetwork>>>,
    pub basis_graph: Option<Arc<BasisGraph>>,
    pub profile: Option<Arc<Profile>>,
    pub normal_schema_transformations: Option<HashMap<Lineage, Arc<SchemaTransformation>>>,
    pub translation_schema_transformations: Option<HashMap<Lineage, Arc<SchemaTransformation>>>,
    pub document: Option<Document>,
    pub translation_schema: Option<Arc<Schema>>,
}

impl MetaContext {
    pub fn new() -> Self {
        MetaContext {
            contexts: None,
            graph_root: None,
            basis_nodes: None,
            basis_networks: None,
            basis_graph: None,
            profile: None,
            normal_schema_transformations: None,
            translation_schema_transformations: None,
            document: None,
            translation_schema: None,
        }
    }

    pub fn get_basis_network_by_subgraph_hash(
        &self,
        subgraph_hash: &String
    ) -> Result<Option<Arc<BasisNetwork>>, Errors> {
        log::trace!("In get_basis_network_by_subgraph_hash");
        
        let basis_networks = self.basis_networks.as_ref().unwrap();

        for basis_network in basis_networks.values() {
            if basis_network.subgraph_hash == *subgraph_hash {
                return Ok(Some(Arc::clone(&basis_network)));
            }
        }

        Ok(None)
    }

    pub fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<Arc<BasisNode>>, Errors> {
        log::trace!("In get_basis_node_by_lineage");

        let basis_nodes = self.basis_nodes.as_ref().unwrap();

        for basis_node in basis_nodes.values() {
            if basis_node.lineage == *lineage {
                return Ok(Some(Arc::clone(&basis_node)));
            }
        }

        Ok(None)
    }

    pub fn update_document(
        &mut self,
        document: Document
    ) {
        self.document = Some(document);
    }

    pub fn update_normal_schema_transformations(
        &mut self,
        schema_transformations: HashMap<Lineage, Arc<SchemaTransformation>>
    ) {
        self.normal_schema_transformations = Some(schema_transformations);
    }

    pub fn update_translation_schema_transformations(
        &mut self,
        schema_transformations: HashMap<Lineage, Arc<SchemaTransformation>>
    ) {
        self.translation_schema_transformations = Some(schema_transformations);
    }

    pub fn update_translation_schema(
        &mut self,
        schema: Schema
    ) {
        self.translation_schema = Some(Arc::new(schema));
    }

    pub fn update_profile(&mut self, profile: Arc<Profile>) {
        self.profile = Some(profile);
    }

    pub fn update_data_structures(&mut self, contexts: HashMap<ID, Arc<Context>>, graph_root: Graph) {
        self.contexts = Some(contexts);
        self.graph_root = Some(graph_root);
    }

    pub fn update_basis_graph(&mut self, graph: Arc<BasisGraph>) {
        self.basis_graph = Some(graph);
    }

    pub fn update_basis_nodes(&mut self, nodes: HashMap<ID, Arc<BasisNode>>) {
        self.basis_nodes = Some(nodes);
    }

    pub fn update_basis_networks(&mut self, networks: HashMap<ID, Arc<BasisNetwork>>) {
        self.basis_networks = Some(networks);
    }

    pub fn get_original_document(&self) -> String {
        log::trace!("In get_original_document");

        let mut document = String::new();
        let mut visited_lineages: HashSet<Lineage> = HashSet::new();
        let root_node = self.graph_root.clone().unwrap();

        traverse_for_condensed_document(
            self,
            Arc::clone(&root_node),
            &mut visited_lineages,
            &mut document,
        );

        document
    }

    pub fn to_document(
        &self,
        document_format: &Option<DocumentFormat>,
    ) -> Result<Document, Errors> {
        log::trace!("In to_document");

        let graph_root = self.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?;
        let basis_graph = self.basis_graph.clone().unwrap();

        let mut result: HashMap<String, Value> = HashMap::new();
        let mut inner_schema: HashMap<String, SchemaNode> = HashMap::new();

        process_network(
            &self,
            graph_root,
            &mut result,
            &mut inner_schema,
            &basis_graph.lineage,
        )?;

        let data = {
            match serde_json::to_string(&result) {
                Ok(json_string) => json_string,
                Err(e) => panic!("Error serializing to JSON: {}", e),
            }
        };

        let schema = Schema {
            id: ID::new(),
            name: basis_graph.name.clone(),
            description: basis_graph.description.clone(),
            lineage: basis_graph.lineage.clone(),
            properties: inner_schema,
        };

        let document = Document {
            document_type: DocumentType::Json,
            metadata: DocumentMetadata {
                origin: None,
                date: None,
            },
            data,
            schema: Some(schema)
        };

        Ok(document)
    }
}

fn traverse_for_condensed_document(
    meta_context: &MetaContext,
    current_node: Graph,
    visited_lineages: &mut HashSet<Lineage>,
    document: &mut String
) {
    let lock = read_lock!(current_node);
    let current_id = lock.id.clone();
    let current_context = meta_context.contexts.clone().unwrap();
    let current_context = current_context.get(&current_id).unwrap();
    let current_lineage = current_context.lineage.clone();
    let document_node = current_context.document_node.clone();

    let should_render = !visited_lineages.contains(&current_lineage);

    visited_lineages.insert(current_lineage.clone());

    if should_render {
        let (a, _) = read_lock!(document_node).to_string_components();

        document.push_str(&a);
    }

    for child in &lock.children {
        traverse_for_condensed_document(
            &meta_context,
            Arc::clone(child),
            visited_lineages,
            document
        );
    }

    if should_render {
        let (_, b) = read_lock!(document_node).to_string_components();

        document.push_str(b.as_deref().unwrap_or(""));
    }
}

fn process_network(
    meta_context: &MetaContext,
    graph: Graph,
    result: &mut HashMap<String, Value>,
    schema: &mut HashMap<String, SchemaNode>,
    schema_lineage: &Lineage,
) -> Result<(), Errors> {
    log::trace!("In process_network");

    let contexts = meta_context.contexts.clone().unwrap();

    let mut queue = VecDeque::new();
    queue.push_back(graph.clone());

    let mut processed_child_ids = HashSet::new();

    while let Some(current) = queue.pop_front() {
        let (context_id, children) = {
            let read_lock = read_lock!(current);
            (read_lock.id.clone(), read_lock.children.clone())
        };

        let context = contexts.get(&context_id).unwrap().clone();

        process_node(
            meta_context,
            context.clone(),
            result,
            schema,
            schema_lineage,
        )?;

        for (index, child) in children.iter().enumerate() {
            let child_id = {
                let child_lock = read_lock!(child);
                child_lock.id.clone()
            };

            if processed_child_ids.contains(&child_id) {
                continue;
            }

            let child_subgraph_hash = {
                let child_lock = read_lock!(child);
                child_lock.subgraph_hash.clone()
            };

            let maybe_basis_network = {
                meta_context.get_basis_network_by_subgraph_hash(
                    &child_subgraph_hash.to_string().unwrap()
                ).expect("Could not get basis network by subgraph hash")
            };

            if let Some(basis_network) = maybe_basis_network {
                log::trace!("Found basis network");

                if !basis_network.is_null_network() {
                    let object_name = basis_network.name.clone();
                    let object_description = basis_network.description.clone();

                    let mut schema_node = SchemaNode::new(
                        &object_name,
                        &object_description,
                        schema_lineage,
                        "object"
                    );

                    {
                        if let Some(schema_transformations) = &meta_context.normal_schema_transformations {
                            if let Some(schema_transformation) = schema_transformations.get(&schema_node.lineage) {
                                log::info!("Found a schema transformation");
                                schema_node = schema_transformation.transform(&schema_node);
                            }
                        }
                    }

                    let mut inner_result: HashMap<String, Value> = HashMap::new();
                    let mut inner_schema: HashMap<String, SchemaNode> = HashMap::new();

                    let mut associated_graphs = match &basis_network.relationship {
                        NetworkRelationship::Association(assoc) => assoc.clone(),
                        _ => Vec::new(),
                    };

                    for subsequent_child in children.iter().skip(index + 1) {
                        let subsequent_child_id = {
                            let subsequent_lock = read_lock!(subsequent_child);
                            subsequent_lock.id.clone()
                        };

                        if processed_child_ids.contains(&subsequent_child_id) {
                            continue;
                        }

                        let subsequent_subgraph_hash = {
                            let subsequent_lock = read_lock!(subsequent_child);
                            subsequent_lock.subgraph_hash.clone()
                        };

                        if associated_graphs.contains(&subsequent_subgraph_hash.to_string().unwrap()) {
                            process_network(
                                meta_context.clone(),
                                subsequent_child.clone(),
                                &mut inner_result,
                                &mut inner_schema,
                                &schema_node.lineage,
                            )?;

                            associated_graphs.retain(|item| item != &subsequent_subgraph_hash.to_string().unwrap());
                            processed_child_ids.insert(subsequent_child_id);
                        }
                    }

                    process_network(
                        meta_context.clone(),
                        child.clone(),
                        &mut inner_result,
                        &mut inner_schema,
                        &schema_node.lineage,
                    )?;

                    let inner_result_value = serde_json::to_value(inner_result)
                        .expect("Failed to serialize inner result");

                    if let Some(existing_object) = result.get_mut(&schema_node.name) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value);
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value
                            ]);
                        }

                        let mut existing_schema_node = schema.get_mut(&schema_node.name).unwrap();
                        existing_schema_node.data_type = "array".to_string();
                    } else {
                        schema_node.properties = inner_schema;
                        schema.insert(schema_node.name.clone(), schema_node.clone());
                        result.insert(schema_node.name.clone(), inner_result_value);
                    }

                    processed_child_ids.insert(child_id);
                } else {
                    queue.push_back(child.clone());
                }
            } else {
                queue.push_back(child.clone());
            }
        }
    }

    Ok(())
}

fn process_node(
    meta_context: &MetaContext,
    context: Arc<Context>,
    result: &mut HashMap<String, Value>,
    schema: &mut HashMap<String, SchemaNode>,
    schema_lineage: &Lineage,
) -> Result<(), Errors> {
    log::trace!("In process_node");

    let maybe_basis_node = {
        meta_context.get_basis_node_by_lineage(&context.lineage)
            .expect("Could not get basis node by lineage")
    };

    if let Some(basis_node) = maybe_basis_node {
        let json_nodes: Vec<JsonNode> = basis_node.transformations
            .clone()
            .into_iter()
            .map(|transformation| {
                transformation.transform(Arc::clone(&context.data_node))
                    .expect("Could not transform data node")
            })
            .collect();

        for json_node in json_nodes.into_iter() {
            let json = json_node.json;
            let key = json.key.clone();
            let trimmed_value = json!(json.value.trim().to_string());

            let mut schema_node = SchemaNode::new(
                &key,
                &json_node.description,
                schema_lineage,
                "string"
            );

            {
                if let Some(schema_transformations) = &meta_context.normal_schema_transformations {
                    if let Some(schema_transformation) = schema_transformations.get(&schema_node.lineage) {
                        log::info!("Found a schema transformation");
                        schema_node = schema_transformation.transform(&schema_node);
                    }
                }
            }

            if let Some(existing_value) = result.get_mut(&schema_node.name) {
                if let Value::Array(ref mut arr) = existing_value {
                    arr.push(trimmed_value);
                } else {
                    *existing_value = json!(vec![existing_value.clone(), trimmed_value]);
                }

                schema_node.data_type = "array".to_string();

                schema.insert(schema_node.name.clone(), schema_node);
            } else {
                result.insert(schema_node.name.clone(), trimmed_value);
                schema.insert(schema_node.name.clone(), schema_node);
            }
        }
    }

    Ok(())
}
