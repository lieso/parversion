use ego_tree::NodeRef;
use scraper::{Html, Node};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string, Map, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use xmltree::Element;

use crate::classification::Classification;
use crate::context::Context;
use crate::data_node::DataNode;
use crate::document_format::DocumentFormat;
use crate::document_node::DocumentNode;
use crate::graph_node::Graph;
use crate::graph_node::GraphNode;
use crate::hash::Hash;
use crate::json_node::JsonNode;
use crate::prelude::*;
use crate::profile::Profile;
use crate::provider::Provider;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::transformation::{
    CanonicalizationTransformation,
    RelationshipTransformation,
    TraversalTransformation,
};
use crate::network_relationship::NetworkRelationshipType;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentType {
    Json,
    PlainText,
    JavaScript,
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
}

impl Document {
    pub fn from_normalized_graph(
        meta_context: Arc<RwLock<MetaContext>>,
        document_format: &DocumentFormat,
    ) -> Result<Self, Errors> {
        log::trace!("In from_normalized_graph");

        match document_format.format_type {
            DocumentType::Json => Self::from_normalized_graph_json(Arc::clone(&meta_context)),
            DocumentType::PlainText => unimplemented!(),
            DocumentType::JavaScript => unimplemented!(),
            DocumentType::Xml => unimplemented!(),
            DocumentType::Html => unimplemented!(),
        }
    }

    fn from_normalized_graph_json(
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<Self, Errors> {
        log::trace!("In from_normalized_graph_json");

        let graph_root = read_lock!(meta_context).normal_graph_root.clone().unwrap();

        let mut result: Map<String, Value> = Map::new();

        fn recurse(
            meta_context: Arc<RwLock<MetaContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>,
        ) {
            let contexts = {
                let lock = read_lock!(meta_context);
                lock.normal_contexts.clone().unwrap()
            };

            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let network_name = &context.network_name;
            let network_description = &context.network_description;
            let data_node = &context.data_node;
            let json_nodes: Vec<JsonNode> = data_node.to_json_nodes();

            for json_node in json_nodes {
                let json = json_node.json;
                let value = json!(json.value.trim().to_string());
                result.insert(json.key, value);
            }

            for child in &read_lock!(graph_node).children {
                let child_context = contexts.get(&read_lock!(child).id).unwrap();

                if let Some(child_network_name) = &child_context.network_name {
                    log::debug!("child_network_name: {}", child_network_name);

                    let mut inner_result: Map<String, Value> = Map::new();

                    recurse(
                        Arc::clone(&meta_context),
                        Arc::clone(&child),
                        &mut inner_result
                    );

                    let inner_result_value = Value::Object(inner_result.clone());

                    if let Some(existing_object) = result.get_mut(child_network_name) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }
                    } else {
                        result.insert(child_network_name.clone(), inner_result_value);
                    }

                } else {
                    recurse(
                        Arc::clone(&meta_context),
                        Arc::clone(&child),
                        result
                    );
                }
            }
        }

        recurse(
            Arc::clone(&meta_context),
            Arc::clone(&graph_root),
            &mut result,
        );

        let document = Document {
            document_type: DocumentType::Json,
            data: serde_json::to_string_pretty(&result).expect("Could not make a JSON string"),
            metadata: DocumentMetadata {
                origin: None,
                date: None,
            },
        };

        Ok(document)
    }

    pub fn get_contexts(
        &self,
        meta_context: Arc<RwLock<MetaContext>>,
        metadata: &Metadata,
    ) -> Result<
        (
            HashMap<ID, Arc<Context>>, // context
            Arc<RwLock<GraphNode>>,    // graph root
        ),
        Errors,
    > {
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
            let data_node = Arc::new(DataNode::new(
                profile.meaningful_fields.clone().unwrap(),
                &profile.hash_transformation.clone().unwrap(),
                read_lock!(document_node).get_fields(),
                read_lock!(document_node).get_description(),
                parent_lineage,
            ));

            let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
                Arc::clone(&data_node),
                parents.clone(),
            )));

            let context = Arc::new(Context {
                id: ID::new(),
                acyclic_lineage: data_node.lineage.acyclic(),
                lineage: data_node.lineage.clone(),
                indexed_lineages: Arc::new(RwLock::new(Vec::new())),
                basis_lineage: Arc::new(RwLock::new(None)),
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
                    .enumerate()
                    .map(|(_child_index, child)| {
                        recurse(
                            Arc::new(RwLock::new(child)),
                            data_nodes,
                            &data_node.lineage,
                            contexts,
                            vec![Arc::clone(&graph_node)],
                            profile,
                        )
})
                    .collect();

                let mut write_lock = graph_node.write().unwrap();

                let child_hashes: Vec<Hash> = children
                    .iter()
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

        let origin_hash = Hash::from_str(&metadata.origin);
        let initial_lineage = Lineage::new().with_hash(origin_hash);

        let graph_root = recurse(
            Arc::clone(&document_root),
            &mut data_nodes,
            &initial_lineage,
            &mut contexts,
            Vec::new(),
            &profile,
        );

        let mut seen: HashSet<ID> = HashSet::new();
        for context in contexts.values() {
            if seen.insert(context.id.clone()) {
                let indexed_lineages = read_lock!(context.graph_node).get_indexed_lineages();
                *context.indexed_lineages.write().unwrap() = indexed_lineages;
            }
        }

        Ok((contexts, graph_root))
    }

    pub fn from_string(
        value: String,
        options: &Options,
        metadata: &Metadata,
    ) -> Result<Self, Errors> {
        if value.trim().is_empty() {
            return Err(Errors::DocumentNotProvided);
        }

        Ok(Document {
            document_type: metadata.document_type.clone().unwrap(),
            metadata: DocumentMetadata {
                origin: options.origin.clone(),
                date: options.date.clone(),
            },
            data: value,
        })
    }

    pub fn to_string(&self) -> String {
        let mut result = serde_json::to_string(self).expect("Could not convert document to string");
        result.push('\n');
        result.push_str(&self.data);

        result
    }

    pub fn get_document_node(&self) -> Result<DocumentNode, Errors> {
        log::trace!("In document/get_document_node");

        if let Some(dom) = self.to_dom() {
            let mut xml = String::from("");

            // TODO: do we want to do anything with this?
            let mut extracted_docs: Vec<Document> = Vec::new();

            walk(&mut xml, dom.tree.root(), 0, &mut extracted_docs);

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
        provider: Arc<P>,
    ) -> Result<Profile, Errors> {
        log::trace!("In perform_analysis");

        let features: Option<HashSet<Hash>> = {
            if let Some(dom) = self.to_dom() {
                log::info!("It seems to be possible to parse this document as XML");

                self.document_type = DocumentType::Xml;

                let mut raw_features: HashSet<String> = HashSet::new();

                get_xml_features(dom.tree.root(), &mut String::from(""), &mut raw_features);

                Some(
                    raw_features
                        .iter()
                        .map(|feature| {
                            let mut hash = Hash::new();
                            hash.push(feature).finalize().clear_items();
                            hash.clone()
                        })
                        .collect(),
                )
            } else {
                None
            }
        };

        let features = features.ok_or(Errors::UnexpectedDocumentType)?;

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
    }

    fn to_dom(&self) -> Option<Html> {
        let sanitized = self.data.replace("\n", "");
        Some(Html::parse_document(&sanitized))
    }
}

fn get_xml_features(node: NodeRef<Node>, path: &mut String, features: &mut HashSet<String>) {
    match node.value() {
        Node::Document => {
            for child in node.children() {
                get_xml_features(child, path, features);
            }
        }
        Node::Text(_) => {
            features.insert(format!("{}/text", path));
        }
        Node::Element(element) => {
            let mut new_path = format!("{}/{}", path, element.name());

            for (attr_name, _) in element.attrs() {
                features.insert(format!("{}.{}", new_path, attr_name.trim()));
            }

            for child in node.children() {
                get_xml_features(child, &mut new_path, features);
            }
        }
        _ => {}
    }
}

fn walk(
    xhtml: &mut String,
    node: NodeRef<Node>,
    indent: usize,
    extracted_docs: &mut Vec<Document>,
) {
    let real_indent = " ".repeat(indent * 2);

    match node.value() {
        Node::Document => {
            for child in node.children() {
                walk(xhtml, child, indent, extracted_docs);
            }
        }
        Node::Text(text) => {
            let text_content = text.trim();
            let text = format!("{}{}\n", real_indent, escape_xml(text_content));

            if !text.trim().is_empty() {
                xhtml.push_str(&text);
            }
        }
        Node::Comment(_) => {
            // Ignoring HTML comments
        }
        Node::Element(element) => {
            let tag_name = element.name();

            xhtml.push_str(&format!("{}<{}", real_indent, tag_name));

            for (attr_name, attr_value) in element.attrs() {
                let attr_name = attr_name.trim();
                let attr_value = attr_value.trim();

                let is_html = is_likely_html(attr_value);
                let _is_javascript = false; // TODO: Check if attr_value is valid JavaScript

                if is_html {
                    let html_doc = Document {
                        document_type: DocumentType::Html,
                        data: attr_value.to_string(),
                        metadata: DocumentMetadata {
                            origin: None,
                            date: None,
                        },
                    };
                    extracted_docs.push(html_doc);
                }

                if _is_javascript {
                    // TODO: Parse as JavaScript and create Document
                }

                if !is_html && !_is_javascript {
                    let escaped_attr_value = escape_xml(attr_value);
                    xhtml.push_str(&format!(" {}=\"{}\"", attr_name, escaped_attr_value));
                }
            }

            xhtml.push_str(">\n");

            for child in node.children() {
                walk(xhtml, child, indent + 1, extracted_docs);
            }

            xhtml.push_str(&format!("{}</{}>\n", real_indent, tag_name));
        }
        _ => {}
    }
}

fn is_likely_html(value: &str) -> bool {
    // Quick heuristic checks first
    if value.len() < 3 {
        return false;
    }

    // Check if string contains HTML tag patterns
    if !value.contains('<') || !value.contains('>') {
        return false;
    }

    // Simple regex-like check for tag patterns: <letters...>
    let has_tag_pattern = value
        .chars()
        .collect::<Vec<char>>()
        .windows(3)
        .any(|window| window[0] == '<' && window[1].is_alphabetic());

    if !has_tag_pattern {
        return false;
    }

    // Fallback to parsing and counting element nodes
    let test_doc = Document {
        document_type: DocumentType::Html,
        data: value.to_string(),
        metadata: DocumentMetadata {
            origin: None,
            date: None,
        },
    };

    if let Some(dom) = test_doc.to_dom() {
        let element_count = count_element_nodes(dom.tree.root());
        // If we have more than just the auto-generated wrapper elements (html, head, body)
        // then this is likely real HTML content
        element_count > 3
    } else {
        false
    }
}

fn count_element_nodes(node: NodeRef<Node>) -> usize {
    let mut count = 0;

    match node.value() {
        Node::Element(_) => {
            count += 1;
            for child in node.children() {
                count += count_element_nodes(child);
            }
        }
        Node::Document => {
            for child in node.children() {
                count += count_element_nodes(child);
            }
        }
        _ => {
            for child in node.children() {
                count += count_element_nodes(child);
            }
        }
    }

    count
}

fn escape_xml(data: &str) -> String {
    data.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}
