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

pub fn traverse_for_schema(
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<String, Vec<SchemaNode>>, Errors> {
    log::trace!("In traverse_for_schema");

    let graph_root = {
        let lock = read_lock!(meta_context);
        lock.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?
    };

    let mut schema_nodes: HashMap<String, Vec<SchemaNode>> = HashMap::new();

    process_network_for_schema(
        meta_context.clone(),
        graph_root,
        &mut schema_nodes,
        Vec::new()
    )?;

    log::debug!("*****************************************************************************************************");
    log::debug!("schema_nodes: {:?}", schema_nodes);

   // let mut result: HashMap<ID, Arc<SchemaNode>> = HashMap::new();

   // for (_key, nodes) in schema_nodes {
   //     if (nodes.is_empty()) {
   //         panic!("There shouldn't be an empty vector here");
   //     }

   //     let schema_node = if nodes.len() == 1 {

   //     } else {

   //     };
   // }




    for (_key, nodes) in &schema_nodes {
        log::debug!("------------------------------------------");
        for node in nodes {
            log::debug!("json_path: {:?}", node.json_path);
        }
    }







    delay();

    Ok(schema_nodes)
}

fn process_network_for_schema(
    meta_context: Arc<RwLock<MetaContext>>,
    graph: Graph,
    result: &mut HashMap<String, Vec<SchemaNode>>,
    json_path: Vec<String>
) -> Result<(), Errors> {
    log::trace!("In process_network_for_schema");

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






        let schema_nodes: Vec<SchemaNode> = process_node_for_schema(
            meta_context.clone(),
            context.clone(),
            json_path.clone()
        )?;


        for schema_node in schema_nodes.iter() {

            let json_path_key = schema_node.json_path.clone().concat();
            log::debug!("json_path_key: ${}", json_path_key);

            result.entry(json_path_key)
                .or_insert_with(Vec::new)
                .push(schema_node.clone());

        }







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
                    let new_json_path: Vec<String> = json_path.iter()
                        .cloned()
                        .chain(std::iter::once(object_name.clone()))
                        .collect();

                    let schema_node = SchemaNode {
                        id: ID::new(),
                        name: object_name.clone(),
                        description: "schema node placeholder description".to_string(),
                        json_path: new_json_path.clone(),
                        data_type: "object".to_string()
                    };
                    
                    let json_path_key = new_json_path.clone().concat();
                    result.entry(json_path_key)
                        .or_insert_with(Vec::new)
                        .push(schema_node.clone());


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
                            process_network_for_schema(
                                meta_context.clone(),
                                subsequent_child.clone(),
                                result,
                                new_json_path.clone(),
                            )?;

                            associated_graphs.retain(|item| item != &subsequent_subgraph_hash.to_string().unwrap());
                            processed_child_ids.insert(subsequent_child_id);
                        }
                    }

                    process_network_for_schema(
                        meta_context.clone(),
                        child.clone(),
                        result,
                        json_path.clone(),
                    )?;

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

fn process_node_for_schema(
    meta_context: Arc<RwLock<MetaContext>>,
    context: Arc<Context>,
    json_path: Vec<String>
) -> Result<Vec<SchemaNode>, Errors> {
    log::trace!("In process_node_for_schema");

    let mut schema_nodes: Vec<SchemaNode> = Vec::new();

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

        for json_node in json_nodes.iter() {
            let key = json_node.json.key.clone();

            let schema_node = SchemaNode {
                id: ID::new(),
                name: key.clone(),
                description: "schema node placeholder description".to_string(),
                json_path: json_path.iter()
                    .cloned()
                    .chain(std::iter::once(key.clone()))
                    .collect(),
                data_type: "string".to_string()
            };

            schema_nodes.push(schema_node);
        }
    }

    Ok(schema_nodes)
}

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

pub fn build_document_from_meta_context(
    meta_context: Arc<RwLock<MetaContext>>,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In build_document_from_meta_context");

    let lock = read_lock!(meta_context);
    let graph_root = lock.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?;

    let mut result: HashMap<String, Value> = HashMap::new();

    process_network(
        meta_context.clone(),
        graph_root,
        &mut result
    )?;

    let data = {
        match serde_json::to_string(&result) {
            Ok(json_string) => json_string,
            Err(e) => panic!("Error serializing to JSON: {}", e),
        }
    };

    let document = Document {
        document_type: DocumentType::Json,
        metadata: DocumentMetadata {
            origin: None,
            date: None,
        },
        data,
    };

    Ok(document)
}

fn process_network(
    meta_context: Arc<RwLock<MetaContext>>,
    graph: Graph,
    result: &mut HashMap<String, Value>,
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
                    let mut inner_result: HashMap<String, Value> = HashMap::new();

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
                            )?;

                            associated_graphs.retain(|item| item != &subsequent_subgraph_hash.to_string().unwrap());
                            processed_child_ids.insert(subsequent_child_id);
                        }
                    }

                    process_network(
                        meta_context.clone(),
                        child.clone(),
                        &mut inner_result,
                    )?;

                    let inner_result_value = serde_json::to_value(inner_result)
                        .expect("Failed to serialize inner result");

                    let object_name = basis_network.name.clone();

                    if let Some(existing_object) = result.get_mut(&object_name) {

                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value);
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value
                            ]);
                        }

                    } else {
                        result.insert(object_name, inner_result_value);
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
    result: &mut HashMap<String, Value>
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

            let trimmed_value = json!(json.value.trim().to_string());

            if let Some(existing_value) = result.get_mut(&json.key) {
                if let Value::Array(ref mut arr) = existing_value {
                    arr.push(trimmed_value);
                } else {
                    *existing_value = json!(vec![existing_value.clone(), trimmed_value]);
                }
            } else {
                result.insert(json.key, trimmed_value);
            }
        }
    }

    Ok(())
}
