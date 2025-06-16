use std::collections::{HashSet, HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use serde_json::{json, Value};

use crate::prelude::*;
use crate::meta_context::MetaContext;
use crate::context::{Context};
use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::document::{Document, DocumentType, DocumentMetadata};
use crate::document_format::{DocumentFormat};
use crate::profile::Profile;
use crate::provider::Provider;
use crate::json_node::JsonNode;
use crate::basis_network::{NetworkRelationship};
use crate::schema_node::SchemaNode;

pub fn traverse_document(
    document: Document,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<(
    HashMap<ID, Arc<Context>>, // context
    Arc<RwLock<GraphNode>> // graph root
), Errors> {
    log::trace!("In traverse_document");

    let lock = read_lock!(meta_context);
    let profile = lock.profile.as_ref().ok_or(Errors::ProfileNotProvided)?;

    let document_root = document.get_document_node()?;
    let document_root = Arc::new(RwLock::new(document_root.clone()));

    let mut data_nodes: HashMap<ID, Arc<DataNode>> = HashMap::new();
    let mut contexts: HashMap<ID, Arc<Context>> = HashMap::new();

    fn recurse(
        document_node: Arc<RwLock<DocumentNode>>,
        data_nodes: &mut HashMap<ID, Arc<DataNode>>,
        parent_lineage: &Lineage,
        contexts: &mut HashMap<ID, Arc<Context>>,
        parents: Vec<Arc<RwLock<GraphNode>>>,
        profile: &Profile,
    ) -> Arc<RwLock<GraphNode>> {
        let data_node = Arc::new(
            DataNode::new(
                profile.meaningful_fields.clone().unwrap(),
                &profile.hash_transformation.clone().unwrap(),
                read_lock!(document_node).get_fields(),
                read_lock!(document_node).get_description(),
                parent_lineage,
            )
        );

        let graph_node = Arc::new(RwLock::new(
            GraphNode::from_data_node(
                Arc::clone(&data_node),
                parents.clone(),
            )
        ));

        let context = Arc::new(Context {
            id: ID::new(),
            lineage: data_node.lineage.clone(),
            document_node: Arc::clone(&document_node),
            graph_node: Arc::clone(&graph_node),
            data_node: Arc::clone(&data_node),
        });

        data_nodes.insert(data_node.id.clone(), Arc::clone(&data_node));

        contexts.insert(data_node.id.clone(), Arc::clone(&context));
        contexts.insert(read_lock!(document_node).id.clone(), Arc::clone(&context));
        contexts.insert(read_lock!(graph_node).id.clone(), Arc::clone(&context));

        {
            let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
                .get_children(profile.xml_element_transformation.clone())
                .into_iter()
                .map(|child| {
                    recurse(
                        Arc::new(RwLock::new(child)),
                        data_nodes,
                        &data_node.lineage,
                        contexts,
                        vec![Arc::clone(&graph_node)],
                        profile
                    )
                })
                .collect();

            let mut write_lock = graph_node.write().unwrap();

            let child_hashes: Vec<Hash> = children.iter()
                .map(|child| read_lock!(child).hash.clone())
                .collect();

            let mut subgraph_hash = Hash::from_items(child_hashes.clone());
            let subgraph_hash = subgraph_hash
                .sort()
                .push(write_lock.hash.clone())
                .finalize();

            write_lock.subgraph_hash = subgraph_hash.clone();
            write_lock.children.extend(children);
        }

        graph_node
    }

    let graph_root = recurse(
        Arc::clone(&document_root),
        &mut data_nodes,
        &Lineage::new(),
        &mut contexts,
        Vec::new(),
        &profile
    );

    Ok((contexts, graph_root))
}

pub fn traverse_meta_context(
    meta_context: Arc<RwLock<MetaContext>>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In traverse_meta_context");

    let lock = read_lock!(meta_context);
    let graph_root = lock.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?;
    let basis_graph = lock.basis_graph.clone().unwrap();



    let mut root_schema_node = SchemaNode::new(
        &basis_graph.name,
        &basis_graph.description,
        &basis_graph.lineage,
        "object",
    );



    let mut result: HashMap<String, Value> = HashMap::new();
    let mut inner_schema: HashMap<String, SchemaNode> = HashMap::new();

    process_network(
        meta_context.clone(),
        graph_root,
        &mut result,
        &mut inner_schema,
        &root_schema_node.lineage,
    )?;

    let data = {
        match serde_json::to_string(&result) {
            Ok(json_string) => json_string,
            Err(e) => panic!("Error serializing to JSON: {}", e),
        }
    };

    root_schema_node.properties = inner_schema;

    let mut schema: HashMap<String, SchemaNode> = HashMap::new();
    schema.insert(basis_graph.name.clone(), root_schema_node);

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

fn process_network(
    meta_context: Arc<RwLock<MetaContext>>,
    graph: Graph,
    result: &mut HashMap<String, Value>,
    schema: &mut HashMap<String, SchemaNode>,
    schema_lineage: &Lineage,
) -> Result<(), Errors> {
    log::trace!("In process_network");

    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts.clone().unwrap()
    };

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
            meta_context.clone(),
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
                let lock = read_lock!(meta_context);
                lock.get_basis_network_by_subgraph_hash(
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
                        let lock = read_lock!(meta_context);
                        if let Some(schema_transformations) = &lock.schema_transformations {
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
    meta_context: Arc<RwLock<MetaContext>>,
    context: Arc<Context>,
    result: &mut HashMap<String, Value>,
    schema: &mut HashMap<String, SchemaNode>,
    schema_lineage: &Lineage,
) -> Result<(), Errors> {
    log::trace!("In process_node");

    let maybe_basis_node = {
        let lock = read_lock!(meta_context);
        lock.get_basis_node_by_lineage(&context.lineage)
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
                let lock = read_lock!(meta_context);
                if let Some(schema_transformations) = &lock.schema_transformations {
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
