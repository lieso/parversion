use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use xmltree::{Element};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashSet, HashMap};

use crate::prelude::*;
use crate::document_node::{DocumentNode};
use crate::provider::Provider;
use crate::profile::Profile;
use crate::hash::{
    Hash,
};
use crate::schema_node::SchemaNode;
use crate::schema::Schema;
use crate::graph_node::{GraphNode};
use crate::context::{Context};
use crate::data_node::DataNode;
use crate::document_format::DocumentFormat;

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
    pub fn apply_schema_transformations(
        &self,
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<Document, Errors> {
        log::trace!("In apply_schema_transformations");

        unimplemented!()
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
