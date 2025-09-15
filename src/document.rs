use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value, Map};
use xmltree::{Element};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashSet, HashMap, VecDeque};

use crate::prelude::*;
use crate::document_node::{DocumentNode};
use crate::provider::Provider;
use crate::profile::Profile;
use crate::hash::{
    Hash,
};
use crate::schema_node::{SchemaNode, arrayify_schema_node};
use crate::schema::Schema;
use crate::graph_node::{GraphNode};
use crate::context::{Context};
use crate::data_node::DataNode;
use crate::document_format::DocumentFormat;
use crate::path::Path;
use crate::basis_network::{NetworkRelationship};
use crate::json_node::JsonNode;
use crate::basis_graph::BasisGraph;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::graph_node::{Graph};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentType {
    Json,
    PlainText,
    Xml,
    Html,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub origin: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub document_type: DocumentType,
    #[serde(skip_serializing)]
    pub data: String,
    pub metadata: DocumentMetadata,
    pub schema: Option<Schema>,
}

impl Document {
    pub fn from_basis_transformations(
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<Self, Errors> {
        log::trace!("In from_basis_transformations");

        let graph_root = {
            let lock = read_lock!(meta_context);
            lock.graph_root.clone().ok_or(Errors::GraphRootNotProvided)?
        };
        let basis_graph: Arc<BasisGraph> = {
            let lock = read_lock!(meta_context);
            lock.basis_graph.clone().ok_or(Errors::BasisGraphNotFound)?
        };

        let mut result: HashMap<String, Value> = HashMap::new();
        let mut inner_schema: HashMap<String, SchemaNode> = HashMap::new();
        let path: Path = Path::from_str(&basis_graph.name);

        process_network(
            Arc::clone(&meta_context),
            graph_root,
            &mut result,
            &mut inner_schema,
            &basis_graph.lineage,
            &path,
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

    pub fn from_schema_transformations(
        meta_context: Arc<RwLock<MetaContext>>,
        document_version: DocumentVersion
    ) -> Result<Self, Errors> {
        log::trace!("In from_schema_transformations");

        let document: Arc<Document> = {
            let lock = read_lock!(meta_context);
            lock.get_document(document_version).clone().ok_or(Errors::DocumentVersionNotFound)?
        };

        match document.document_type {
            DocumentType::Json => {
                match serde_json::from_str::<Value>(&document.data) {
                    Ok(json_value) => {
                        log::debug!("Parsed JSON: {:?}", json_value);

                        let schema_nodes = document.schema.clone().unwrap().collect_schema_nodes();

                        apply_schema_transformations_json(
                            Arc::clone(&meta_context),
                            &schema_nodes,
                            &json_value,
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to parse JSON: {}", e);
                        Err(Errors::UnexpectedError)
                    }
                }
            }
            _ => {
                log::error!("Unexpected document type: {:?}", document.document_type);
                unimplemented!()
            }
        }
    }

    pub fn get_contexts(
        &self,
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<(
        HashMap<ID, Arc<Context>>, // context
        Arc<RwLock<GraphNode>> // graph root
    ), Errors> {
        log::trace!("In get_contexts");

        let lock = read_lock!(meta_context);
        let profile = lock.profile.as_ref().ok_or(Errors::ProfileNotProvided)?;

        let document_root = self.get_document_node()?;
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

    pub fn from_string(
        value: String,
        options: &Option<Options>,
    ) -> Result<Self, Errors> {
        if value.trim().is_empty() {
            return Err(Errors::DocumentNotProvided);
        }

        Ok(Document {
            document_type: DocumentType::PlainText,
            metadata: DocumentMetadata {
                origin: options.as_ref().and_then(|opts| opts.origin.clone()),
                date: options.as_ref().and_then(|opts| opts.date.clone()),
            },
            data: value,
            schema: None,
        })
    }

    pub fn to_string(&self, document_format: &Option<DocumentFormat>) -> String {
        let mut result = serde_json::to_string(self).expect("Could not convert document to string");
        result.push('\n');
        result.push_str(&self.data);

        result
    }

    pub fn get_document_node(&self) -> Result<DocumentNode, Errors> {
        log::trace!("In document/get_document_node");

        if let Some(dom) = self.to_dom() {

            let mut xml = String::from("");
            walk(&mut xml, &dom.document, 0);

            let reader = std::io::Cursor::new(xml);

            match Element::parse(reader) {
                Ok(element) => Ok(DocumentNode::new(xmltree::XMLNode::Element(element))),
                Err(e) => {
                    log::error!("Could not parse XML: {}", e);

                    Err(Errors::XmlParseError)
                }
            }
        } else {
            unimplemented!()
        }
    }

    pub async fn perform_analysis<P: Provider>(
        &mut self,
        provider: Arc<P>
    ) -> Result<Profile, Errors> {
        log::trace!("In document/perform_analysis");

        if let Some(dom) = self.to_dom() {
            log::info!("It seems to be possible to parse this document as XML");

            self.document_type = DocumentType::Xml;

            let mut features: HashSet<String> = HashSet::new();

            get_xml_features(
                &dom.document,
                &mut String::from(""),
                &mut features,
            );

            let features: HashSet<Hash> = features.iter().map(|feature| {
                let mut hash = Hash::new();
                hash.push(feature).finalize().clear_items();
                hash.clone()
            }).collect();

            if let Some(profile) = provider.get_profile(&features).await? {
                log::info!("Found a profile");

                if profile.xml_element_transformation.is_none() {
                    log::info!("Profile provided but xml transformation missing");
                    unimplemented!();
                }

                if profile.hash_transformation.is_none() {
                    log::info!("Profile provided but hash transformation is missing");
                    unimplemented!();
                }

                Ok(profile)
            } else {
                log::info!("Profile not provided, we will create a new one");

                let profile = Profile::create_profile(&features).await?;

                provider.save_profile(&profile).await?;

                Ok(profile)
            }
        } else {
             Err(Errors::UnexpectedDocumentType)
        }
    }

    fn to_dom(&self) -> Option<RcDom> {
        let sanitized = self.data.replace("\n", "");

        parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut sanitized.as_bytes())
            .ok()
    }
}

fn get_xml_features(
    node: &Handle,
    path: &mut String,
    features: &mut HashSet<String>,
) {
    match &node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                get_xml_features(child, path, features);
            }
        }
        NodeData::Text { .. } => {
            features.insert(format!("{}/text", path));
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let mut new_path = format!("{}/{}", path, name.local);

            for attr in attrs.borrow().iter() {
                let attr_name = attr.name.local.trim();
                features.insert(format!("{}.{}", new_path, attr_name));
            }

            for child in node.children.borrow().iter() {
                get_xml_features(child, &mut new_path, features);
            }
        }
        _ => {}
    }
}

fn walk(xhtml: &mut String, handle: &Handle, indent: usize) {
    let node = handle;
    let real_indent = " ".repeat(indent * 2);

    match node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                walk(xhtml, child, indent);
            }
        }
        NodeData::Text { ref contents } => {
            let contents = &contents.borrow();
            let text = format!("{}{}\n", real_indent, escape_xml(contents.trim()));

            if !text.trim().is_empty() {
                xhtml.push_str(&text);
            }
        },
        NodeData::Comment { ref contents } => {
            log::warn!("Ignoring HTML comment: {}", contents.escape_default());
        },
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let tag_name = &name.local;

            xhtml.push_str(&format!("{}<{}", real_indent, tag_name));

            for attr in attrs.borrow().iter() {
                let attr_name = &*attr.name.local.trim();
                let attr_value = escape_xml(&*attr.value.trim());

                xhtml.push_str(&format!(" {}=\"{}\"", attr_name.escape_default(), attr_value));
            }

            xhtml.push_str(">\n");

            for child in node.children.borrow().iter() {
                walk(xhtml, child, indent + 1);
            }

            xhtml.push_str(&format!("{}</{}>\n", real_indent, tag_name));
        },
        _ => {}
    }
}

fn escape_xml(data: &str) -> String {
    data.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

fn apply_schema_transformations_json(
    meta_context: Arc<RwLock<MetaContext>>,
    schema_nodes: &HashMap<Lineage, SchemaNode>,
    json: &Value
) -> Result<Document, Errors> {

    let mut result: Map<String, Value> = Map::new();

    let basis_graph: Arc<BasisGraph> = {
        let lock = read_lock!(meta_context);
        lock.basis_graph.clone().ok_or(Errors::BasisGraphNotFound)?
    };
    let start_path: Path = Path::from_str(&basis_graph.name);

    fn recurse(
        meta_context: Arc<RwLock<MetaContext>>,
        value: &Value,
        parent_lineage: &Lineage,
        schema_nodes: &HashMap<Lineage, SchemaNode>,
        result: &mut Map<String, Value>,
        path: &Path,
    ) {
        match value {
            Value::Array(arr) => {
                for (index, v) in arr.iter().enumerate() {
                    recurse(
                        Arc::clone(&meta_context),
                        v,
                        parent_lineage,
                        schema_nodes,
                        result,
                        &path.with_index_segment(index),
                    );
                }
            }
            Value::Object(obj) => {
                for (k, v) in obj {
                    let lineage = parent_lineage.with_hash(Hash::from_str(k));

                    recurse(
                        Arc::clone(&meta_context),
                        v,
                        &lineage,
                        schema_nodes,
                        result,
                        &path.with_key_segment(k.clone())
                    );
                }
            },
            _ => {
                let schema_node: SchemaNode = {
                    let current_schema_node = schema_nodes.get(parent_lineage).unwrap();

                    let lock = read_lock!(meta_context);

                    if let Some(schema_transformations) = &lock.schema_transformations {
                        if let Some(transformation) = schema_transformations.get(parent_lineage) {
                            transformation.transform(current_schema_node)
                        } else {
                            current_schema_node.clone()
                        }
                    } else {
                        current_schema_node.clone()
                    }
                };

                log::debug!("path: {:?}", path.to_string());
                log::debug!("schema node path: {:?}", schema_node.path.to_string());

                schema_node.path.insert_into_map(
                    result,
                    schema_node.name.to_string(),
                    value.to_string()
                );
            }
        }
    }

    recurse(
        Arc::clone(&meta_context),
        &json,
        &basis_graph.lineage,
        schema_nodes,
        &mut result,
        &start_path,
    );

    log::debug!("result: {:?}", result);

    unimplemented!()
}

fn process_network(
    meta_context: Arc<RwLock<MetaContext>>,
    graph: Graph,
    result: &mut HashMap<String, Value>,
    schema: &mut HashMap<String, SchemaNode>,
    schema_lineage: &Lineage,
    path: &Path,
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
            Arc::clone(&meta_context),
            context.clone(),
            result,
            schema,
            schema_lineage,
            path,
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
                        &path.with_key_segment(object_name.to_string()),
                        "object",
                    );

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
                                &schema_node.path,
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
                        &schema_node.path,
                    )?;

                    let inner_result_value = serde_json::to_value(inner_result)
                        .expect("Failed to serialize inner result");

                    if let Some(existing_object) = result.get_mut(&schema_node.name) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }

                        let mut existing_schema_node = schema.get_mut(&schema_node.name).unwrap();
                        if existing_schema_node.data_type != "array" {
                            let last_path_segment_id = existing_schema_node
                                .get_last_path_segment()
                                .unwrap()
                                .id
                                .clone();

                            arrayify_schema_node(existing_schema_node, &last_path_segment_id);
                        }
                    } else {
                        schema_node.properties = inner_schema;
                        schema.insert(schema_node.name.clone(), schema_node.clone());
                        result.insert(schema_node.name.clone(), inner_result_value.clone());
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
    path: &Path,
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
                path,
                "string",
            );

            if let Some(existing_value) = result.get_mut(&schema_node.name) {
                if let Value::Array(ref mut arr) = existing_value {
                    arr.push(trimmed_value.clone());
                } else {
                    *existing_value = json!(vec![existing_value.clone(), trimmed_value.clone()]);
                }

                schema_node.data_type = "array".to_string();

                schema.insert(schema_node.name.clone(), schema_node);
            } else {
                result.insert(schema_node.name.clone(), trimmed_value.clone());
                schema.insert(schema_node.name.clone(), schema_node);
            }
        }
    }

    Ok(())
}
