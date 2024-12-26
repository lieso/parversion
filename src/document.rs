use serde::{Serialize, Deserialize};
use xmltree::{Element, XMLNode};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::HashMap;

use crate::prelude::*;
use crate::data_node::{DataNode};

pub type DocumentNode = XMLNode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DocumentType {
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

        if let Ok(xml) = string_to_xml(value) {
            let document = Document {
                document_type: DocumentType::Xml,
                metadata: DocumentMetadata {
                    origin: options.as_ref().and_then(|opts| opts.origin.clone()),
                    date: options.as_ref().and_then(|opts| opts.date.clone()),
                },
                data: xml,
            };

            Ok(document)
        } else {
            Err(Errors::UnexpectedDocumentType)
        }
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

    pub async fn perform_analysis(&mut self) -> Result<(), Errors> {
        // provide sample
        // ask if it uses meaningful class namres
        // create transformation if it doesn't


        // identify clusters
        // ask if cluster is discardable
        // less total inference required
        // e.g. navigation bars are clusted away from contetn
        unimplemented!()
    }

    pub fn apply_transformations(&self) {
        unimplemented!()
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

fn walk(xhtml: &mut String, handle: &Handle, indent: usize) {
    let node = handle;
    let real_indent = " ".repeat(indent * 2);

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
