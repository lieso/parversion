use serde::{Serialize, Deserialize};
use xmltree::{Element, XMLNode};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::document_node::{DocumentNode};
use crate::provider::Provider;
use crate::profile::Profile;
use crate::transformation::XMLElementTransformation;
use crate::hash::{Hash};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentType {
    JSON,
    PLAIN_TEXT,
    XML,
    HTML,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub origin: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub document_type: DocumentType,
    pub metadata: DocumentMetadata,
    pub data: String,
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
            document_type: DocumentType::PLAIN_TEXT,
            metadata: DocumentMetadata {
                origin: options.as_ref().and_then(|opts| opts.origin.clone()),
                date: options.as_ref().and_then(|opts| opts.date.clone()),
            },
            data: value,
        })
    }

    pub fn to_string(self) -> String {
        self.data.clone()
    }

    pub fn get_document_node(&self) -> Result<DocumentNode, Errors> {
        log::trace!("In document/get_document_node");

        if let Some(dom) = self.to_dom() {

            let mut xml = String::from("");
            walk(&mut xml, &dom.document, 0);



            let mut reader = std::io::Cursor::new(xml);

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
        provider: &P
    ) -> Result<Profile, Errors> {
        log::trace!("In document/perform_analysis");

        if let Some(dom) = self.to_dom() {
            log::info!("It seems to be possible to parse this document as XML");

            self.document_type = DocumentType::XML;

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
                    log::info!("Profile provided by hash transformation is missing");
                    unimplemented!();
                }

                Ok(profile)
            } else {
                log::info!("Profile not provided, we will create a new one");
                unimplemented!();
            }
        } else {
             Err(Errors::UnexpectedDocumentType)
        }
    }

    //pub fn apply_transformations(
    //    &mut self,
    //    profile: &Profile
    //) -> Result<(), Errors> {
    //    log::trace!("In document/apply_transformations");

    //    assert!(profile.document_transformations.clone().is_some());
    //    assert!(!profile.document_transformations.clone().unwrap().is_empty());

    //    match self.document_type {
    //        DocumentType::XML => {
    //            if let Some(dom) = self.to_dom() {
    //                let mut xml: String = String::from("");
    //                let transformations = &profile.document_transformations
    //                    .clone().unwrap();

    //                walk_transform(&mut xml, &dom.document, 0, transformations);

    //                log::debug!("Transformed XML document: {}", xml);

    //                self.data = xml;

    //                Ok(())
    //            } else {
    //                 Err(Errors::UnexpectedDocumentType)
    //            }
    //        },
    //        _ => Err(Errors::UnexpectedDocumentType),
    //    }
    //}

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

//fn walk_transform(
//    xml: &mut String,
//    node: &Handle,
//    indent_factor: usize,
//    transformations: &Vec<DocumentTransformation>
//) {
//    let indentation = " ".repeat(indent_factor * 2);
//
//    let xml_element_transformations: Vec<XMLElementTransformation> = transformations.into_iter().filter_map(|transformation| {
//        match transformation {
//            DocumentTransformation::XMLElementTransformation(t) => Some(t.clone()),
//            _ => None,
//        }
//    }).collect();
//
//    match node.data {
//        NodeData::Document => {
//            for child in node.children.borrow().iter() {
//                walk_transform(xml, child, indent_factor, transformations);
//            }
//        }
//        NodeData::Text { ref contents } => {
//            let contents = &contents.borrow();
//            let text = format!("{}{}\n", indentation, escape_xml(contents.trim()));
//
//            if !text.trim().is_empty() {
//                xml.push_str(&text);
//            }
//        },
//        NodeData::Comment { ref contents } => {
//            log::warn!("Ignoring HTML comment: {}", contents.escape_default());
//        },
//        NodeData::Element {
//            ref name,
//            ref attrs,
//            ..
//        } => {
//            let mut element: Option<String> = Some(name.local.to_string());
//            let mut attributes: HashMap<String, String>  = HashMap::new();
//
//            for attr in attrs.borrow().iter() {
//                let attr_name = attr.name.local.trim().clone();
//                let attr_value = escape_xml(&attr.value.trim().to_string());
//
//                attributes.insert(attr_name.to_string(), attr_value);
//            }
//
//            log::info!("Applying XML element transformations...");
//
//            for transformation in xml_element_transformations.iter() {
//                let (transformed_element, transformed_attributes) = transformation.transform(
//                    element.unwrap().clone(),
//                    attributes.clone()
//                );
//
//                attributes = transformed_attributes;
//
//                if let Some(transformed_element) = transformed_element {
//                    element = Some(transformed_element);
//                } else {
//                    log::info!("Transformation has eliminated an element, no further transfomations will be applied");
//                    element = None;
//                    break;
//                }
//            }
//
//            log::info!("Done applying XML element transformations.");
//
//            if let Some(element) = element {
//                xml.push_str(&format!("{}<{}", indentation, element));
//
//                for (attr_name, attr_value) in &attributes {
//                    let value = attributes.get(attr_name).unwrap();
//                    xml.push_str(&format!(" {}=\"{}\"", attr_name, value));
//                }
//
//                xml.push_str(">\n");
//
//                for child in node.children.borrow().iter() {
//                    walk_transform(xml, child, indent_factor + 1, transformations);
//                }
//
//                xml.push_str(&format!("{}</{}>\n", indentation, element));
//            } else {
//                for child in node.children.borrow().iter() {
//                    walk_transform(xml, child, indent_factor + 1, transformations);
//                }
//            }
//        },
//        _ => {}
//    }
//}

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
