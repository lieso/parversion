use serde::{Serialize, Deserialize};
use xmltree::{Element};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashSet, HashMap};
use std::sync::Arc;

use crate::prelude::*;
use crate::document_node::{DocumentNode};
use crate::provider::Provider;
use crate::profile::Profile;
use crate::hash::{
    Hash,
};
use crate::schema_node::SchemaNode;
use crate::schema::Schema;

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
    pub data: String,
    pub metadata: DocumentMetadata,
    pub schema: Option<Schema>,
}

impl Document {
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

    pub fn to_string(self) -> String {
        serde_json::to_string(&self).expect("Could not convert document to string")
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
