use serde::{Serialize, Deserialize};
use serde_json::Value;
use xmltree::{Element, XMLNode};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use quick_js::{Context, JsValue};

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::provider::Provider;
use crate::transformation::{
    Runtime,
    DocumentTransformation,
    XMLElementTransformation,
};

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
            self.document_type = DocumentType::XML;


            log::info!("It seems to be possible to parse this document as XML");




            fn calculate_hash<T: Hash>(t: &T) -> u64 {
                let mut s = DefaultHasher::new();
                t.hash(&mut s);
                s.finish()
            }

            let mut features: HashSet<String> = HashSet::new();

            get_xml_features(&dom.document, String::from(""), &mut features);

            let mut features: HashSet<u64> = features.iter().map(|feature| calculate_hash(feature)).collect();





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

    pub fn apply_transformations(&self) {

        log::debug!("transformations: {:?}", self.transformations);



        if self.transformations.is_empty() {

            panic!("Not expecting there to be zero transformations");

        }



        if let Some(dom) = self.to_dom() {


            for transformation in self.transformations.iter() {

                match transformation {

                    DocumentTransformation::XMLElementTransformation(t) => {

                        let mut xml: String = String::from("");

                        walk_transform(&mut xml, &dom.document, 0, &t);

                        log::debug!("transformed document: {}", xml);



                    }

                }

            }



        }








        unimplemented!()
    }

    fn to_dom(&self) -> Option<RcDom> {
        let sanitized = self.data.replace("\n", "");

        parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut sanitized.as_bytes())
            .ok()
    }
}

fn get_xml_features(node: &Handle, path: String, features: &mut HashSet<String>) {
    match node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                get_xml_features(child, path.clone(), features);
            }
        }
        NodeData::Text { ref contents } => {
            features.insert(format!("{}/text", path));
        },
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let new_path = format!("{}/{}", path, &name.local);

            for attr in attrs.borrow().iter() {
                let attr_name = &*attr.name.local.trim();
                features.insert(format!("{}.{}", new_path, attr_name));
            }

            for child in node.children.borrow().iter() {
                get_xml_features(child, new_path.clone(), features);
            }
        },
        _ => {}
    }
}

fn walk_transform(xml: &mut String, node: &Handle, indent: usize, transformation: &XMLElementTransformation) {
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
                walk_transform(xml, child, indent, transformation);
            }
        }
        NodeData::Text { ref contents } => {
            let contents = &contents.borrow();
            let text = format!("{}{}\n", real_indent, escape_xml(contents.trim()));

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
            let tag_name = &name.local;

            let mut quick_attributes: HashMap<String, String>  = HashMap::new();

            for attr in attrs.borrow().iter() {
                let attr_name = &*attr.name.local.trim().to_string();
                let attr_value = escape_xml(&attr.value.trim().to_string());

                quick_attributes.insert(attr_name.to_string(), attr_value);
            }

            let quick_signature = XMLElementTransformation::get_signature(tag_name.to_string(), quick_attributes.clone());

            log::debug!("quick_signature: {}", quick_signature);

            let quick_code = format!(r#"
{}
{}
JSON.stringify({{ element, attributes }});
"#, quick_signature, &transformation.code);

            log::debug!("quick_code: {}", quick_code);


            let quick_context = Context::new().unwrap();
            let result =  quick_context.eval_as::<String>(&quick_code).unwrap();
            log::debug!("result: {}", result);


            let parsed: Value = serde_json::from_str(&result).unwrap();

            let transformed_element = parsed.get("element").and_then(|e|
                e.as_str().map(String::from));

            let transformed_attributes = parsed["attributes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<String>>();


            if let Some(transformed_element) = transformed_element {

                xml.push_str(&format!("{}<{}", real_indent, transformed_element));


                for attribute in transformed_attributes.iter() {

                    let value = quick_attributes.get(attribute).unwrap();

                    xml.push_str(&format!(" {}=\"{}\"", attribute, value));

                }


                xml.push_str(">\n");

                for child in node.children.borrow().iter() {
                    walk_transform(xml, child, indent + 1, transformation);
                }

                xml.push_str(&format!("{}</{}>\n", real_indent, transformed_element));


            } else {
                for child in node.children.borrow().iter() {
                    walk_transform(xml, child, indent + 1, transformation);
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
