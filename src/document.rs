use serde::{Serialize, Deserialize};
use xmltree::{Element, XMLNode};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::provider::Provider;
use crate::transformation::{
    Runtime,
    DocumentTransformation,
    XMLElementTransformation,
};
use crate::hash::{Hash, FastHash};

pub type DocumentNode = XMLNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub transformations: Vec<DocumentTransformation>,
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
            transformations: Vec::new(),
        })
    }

    pub fn to_string(self) -> String {
        self.data.clone()
    }

    pub fn get_root_node(self) -> (DataNode, Vec<XMLNode>) {
        let mut reader = std::io::Cursor::new(self.data);
        match Element::parse(reader) {
            Ok(element) => {
                Document::document_to_data(xmltree::XMLNode::Element(element), None)
            },
            _ => panic!("Could not parse xml")
        }
    }

    pub fn document_to_data(
        xml_node: XMLNode,
        parent_node: Option<DataNode>,
    ) -> (DataNode, Vec<DocumentNode>) {
        //let context_id = context.register(&xml_node);

        let lineage = match &parent_node {
            Some(node) => &node.lineage,
            None => &Lineage::new(),
        };

        let context_id = ID::new();

        match xml_node {
            XMLNode::Element(element_node) => {
                let mut description = format!("{:?}", element_node);
                description.truncate(20);

                (
                    DataNode::new(
                        context_id,
                        element_node.attributes,
                        description,
                        &lineage
                    ),
                    element_node.children
                )
            },
            XMLNode::Text(text_node) => {
                let mut description = text_node.to_string();
                description.truncate(20);

                (
                    DataNode::new(
                        context_id,
                        HashMap::from([
                            ("text".to_string(), text_node.to_string())
                        ]),
                        description,
                        &lineage
                    ),
                    Vec::new()
                )
            },
            _ => panic!("Unexpected node type")
        }
    }

    pub async fn perform_analysis<P: Provider>(
        &mut self,
        provider: &P
    ) -> Result<(), Errors> {
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
            //let features: HashSet<FastHash> = features.iter().map(|feature| {
            //    FastHash::new().push(feature).finalize().clear_items()
            //}).collect();

            if let Some(document_profile) = provider.get_document_profile(&features).await? {
                log::info!("Document profile provided, we will not proceed with further analysis");

                self.transformations = document_profile.transformations.clone();

                Ok(())
            } else {
                log::info!("Document profile not provided, we will create a new one");
                unimplemented!();
            }
        } else {
             Err(Errors::UnexpectedDocumentType)
        }
    }

    pub fn apply_transformations(&mut self) -> Result<(), Errors> {
        log::trace!("In apply_transformations");

        if self.transformations.is_empty() {
            panic!("Not expecting there to be zero transformations");
        }

        match self.document_type {
            DocumentType::XML => {
                if let Some(dom) = self.to_dom() {
                    let mut xml: String = String::from("");
                    let transformations = self.transformations.clone();

                    walk_transform(&mut xml, &dom.document, 0, &transformations);

                    log::debug!("Transformed XML document: {}", xml);

                    self.data = xml;

                    Ok(())
                } else {
                     Err(Errors::UnexpectedDocumentType)
                }
            },
            _ => Err(Errors::UnexpectedDocumentType),
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

fn escape_xml(data: &str) -> String {
    data.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

fn walk_transform(
    xml: &mut String,
    node: &Handle,
    indent_factor: usize,
    transformations: &Vec<DocumentTransformation>
) {
    let indentation = " ".repeat(indent_factor * 2);

    let xml_element_transformations: Vec<XMLElementTransformation> = transformations.into_iter().filter_map(|transformation| {
        match transformation {
            DocumentTransformation::XMLElementTransformation(t) => Some(t.clone()),
            _ => None,
        }
    }).collect();

    match node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                walk_transform(xml, child, indent_factor, transformations);
            }
        }
        NodeData::Text { ref contents } => {
            let contents = &contents.borrow();
            let text = format!("{}{}\n", indentation, escape_xml(contents.trim()));

            if !text.trim().is_empty() {
                xml.push_str(&text);
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
            let mut element: Option<String> = Some(name.local.to_string());
            let mut attributes: HashMap<String, String>  = HashMap::new();

            for attr in attrs.borrow().iter() {
                let attr_name = attr.name.local.trim().clone();
                let attr_value = escape_xml(&attr.value.trim().to_string());

                attributes.insert(attr_name.to_string(), attr_value);
            }

            log::info!("Applying XML element transformations...");

            for transformation in xml_element_transformations.iter() {
                let (transformed_element, transformed_attributes) = transformation.transform(
                    element.unwrap().clone(),
                    attributes.clone()
                );

                attributes = transformed_attributes;

                if let Some(transformed_element) = transformed_element {
                    element = Some(transformed_element);
                } else {
                    log::info!("Transformation has eliminated an element, no further transfomations will be applied");
                    element = None;
                    break;
                }
            }

            log::info!("Done applying XML element transformations.");

            if let Some(element) = element {
                xml.push_str(&format!("{}<{}", indentation, element));

                for (attr_name, attr_value) in &attributes {
                    let value = attributes.get(attr_name).unwrap();
                    xml.push_str(&format!(" {}=\"{}\"", attr_name, value));
                }

                xml.push_str(">\n");

                for child in node.children.borrow().iter() {
                    walk_transform(xml, child, indent_factor + 1, transformations);
                }

                xml.push_str(&format!("{}</{}>\n", indentation, element));
            } else {
                for child in node.children.borrow().iter() {
                    walk_transform(xml, child, indent_factor + 1, transformations);
                }
            }
        },
        _ => {}
    }
}

fn string_to_xml(value: String) -> Result<String, Errors> {
    let mut xhtml = String::from("");

    let sanitized = value.replace("\n", "");

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut sanitized.as_bytes())
        .unwrap();

    walk(&mut xhtml, &dom.document, 0);

    if xhtml.trim().is_empty() {
        return Err(Errors::UnexpectedDocumentType);
    }

    Ok(xhtml)
}

fn walk(xhtml: &mut String, handle: &Handle, indent_factor: usize) {
    let node = handle;
    let indentation = " ".repeat(indent_factor * 2);

    fn escape_xml(data: &str) -> String {
        data.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&apos;")
    }

    match node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                walk(xhtml, child, indent_factor);
            }
        }
        NodeData::Text { ref contents } => {
            let contents = &contents.borrow();
            let text = format!("{}{}\n", indentation, escape_xml(contents.trim()));

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

            xhtml.push_str(&format!("{}<{}", indentation, tag_name));

            for attr in attrs.borrow().iter() {
                let attr_name = &*attr.name.local.trim();
                let attr_value = escape_xml(&*attr.value.trim());

                xhtml.push_str(&format!(" {}=\"{}\"", attr_name.escape_default(), attr_value));
            }

            xhtml.push_str(">\n");

            for child in node.children.borrow().iter() {
                walk(xhtml, child, indent_factor + 1);
            }

            xhtml.push_str(&format!("{}</{}>\n", indentation, tag_name));
        },
        _ => {}
    }
}

//lazy_static! {
//    pub static ref DOCUMENT_TRANSFORMATIONS: Vec<Transformation> = vec![
//        DocumentTransformation {
//            runtime: Runtime::AWK,
//            description: String::from("Unseen blacklisted attributes"),
//            regex: Regex::new(r#"
//"style", "bgcolor", "border", "cellpadding", "cellspacing",
//"width", "height", "rows", "cols", "wrap",
//"aria-hidden", "size", "op", "lang", "colspan", "rel"
//            "#).unwrap(),
//            expression: String::from(r#"{ print $0 }"#),
//        },
//        DocumentTransformation {
//            runtime: Runtime::AWK,
//            description: String::from("Unseen blacklisted elements"),
//            regex: Regex::new(r#"
//"script", "meta", "link", "iframe", "svg", "style", "noscript"
//            "#).unwrap(),
//            expression: String::from(r#"{ print $0 }"#),
//        },
//    ];
//}
