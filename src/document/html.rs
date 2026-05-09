use ego_tree::NodeRef;
use scraper::{Html as ScraperHtml, Node as ScraperNode};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use xmltree::Element;

use crate::prelude::*;
use crate::context::Context;
use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::GraphNode;
use crate::hash::Hash;
use crate::document::{Document, DocumentType, DocumentMetadata};

pub struct Html;

impl Html {

    pub fn get_contexts(
        meta_context: Arc<RwLock<MetaContext>>,
        metadata: &Metadata,
        data: String
    ) -> Result<
        (
            HashMap<ID, Arc<Context>>, // context
            Arc<RwLock<GraphNode>>,    // graph root
        ),
        Errors,
    > {
        log::trace!("In get_contexts");

        let document_root = Self::get_document_node(data)?;
        let document_root = Arc::new(RwLock::new(document_root.clone()));

        let mut contexts: HashMap<ID, Arc<Context>> = HashMap::new();

        fn recurse(
            document_node: Arc<RwLock<DocumentNode>>,
            parent_lineage: &Lineage,
            contexts: &mut HashMap<ID, Arc<Context>>,
            parents: Vec<Arc<RwLock<GraphNode>>>,
        ) -> Arc<RwLock<GraphNode>> {
            let (hash, lineage, fields, description) = {
                let lock = read_lock!(document_node);
                let hash = lock.get_hash();
                let lineage = parent_lineage.with_hash(hash.clone());
                (hash, lineage, lock.get_fields(), lock.get_description())
            };

            let data_node = Arc::new(DataNode::new(
                hash,
                lineage.clone(),
                fields,
                description,
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

            contexts.insert(data_node.id.clone(), Arc::clone(&context));
            contexts.insert(read_lock!(document_node).id.clone(), Arc::clone(&context));
            contexts.insert(read_lock!(graph_node).id.clone(), Arc::clone(&context));

            {
                let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
                    .get_children()
                    .into_iter()
                    .map(|child| {
                        recurse(
                            Arc::new(RwLock::new(child)),
                            &data_node.lineage,
                            contexts,
                            vec![Arc::clone(&graph_node)],
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
            &initial_lineage,
            &mut contexts,
            Vec::new(),
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

    fn get_document_node(data: String) -> Result<DocumentNode, Errors> {
        log::trace!("In get_document_node");

        if let Some(dom) = to_dom(data) {
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
}

fn to_dom(data: String) -> Option<ScraperHtml> {
    let sanitized = data.replace("\n", "");
    Some(ScraperHtml::parse_document(&sanitized))
}

fn walk(
    xhtml: &mut String,
    node: NodeRef<ScraperNode>,
    indent: usize,
    extracted_docs: &mut Vec<Document>,
) {
    let real_indent = " ".repeat(indent * 2);

    match node.value() {
        ScraperNode::Document => {
            for child in node.children() {
                walk(xhtml, child, indent, extracted_docs);
            }
        }
        ScraperNode::Text(text) => {
            let text_content = text.trim();
            let text = format!("{}{}\n", real_indent, escape_xml(text_content));

            if !text.trim().is_empty() {
                xhtml.push_str(&text);
            }
        }
        ScraperNode::Comment(_) => {
            // Ignoring HTML comments

        }
        ScraperNode::Element(_) => {
            let _ = process_element(node, xhtml, indent, extracted_docs);
        }
        _ => {}
    }
}

fn preprocess_element(tag_name: &str) -> Option<String> {
    match tag_name {
        "svg" | "script" | "iframe" | "input" | "button" => None,
        _ => Some(tag_name.to_string()),
    }
}

fn preprocess_attribute(attr_name: &str, attr_value: &str) -> Option<(String, String)> {
    let attr_name_lower = attr_name.to_lowercase();

    match attr_name_lower.as_str() {
        "aria-describedby" | "aria-controls" | "aria-expanded" | "aria-labelledby"
        | "aria-live" | "aria-level" | "aria-hidden" | "aria-atomic" => None,
        "class" | "style" | "width" | "height" => None,
        "type" | "lang" | "rel" | "role" => None,
        "crossorigin" | "xmlns" => None,
        "tabindex" | "maxlength" => None,
        "fill" => None,
        "target" if attr_value == "_blank" => None,
        _ if attr_value.is_empty() => None,
        _ => Some((attr_name.to_string(), attr_value.to_string())),
    }
}

fn process_element(
    node: NodeRef<ScraperNode>,
    xhtml: &mut String,
    indent: usize,
    extracted_docs: &mut Vec<Document>,
) -> Option<()> {
    let real_indent = " ".repeat(indent * 2);

    if let ScraperNode::Element(element) = node.value() {
        let tag_name = preprocess_element(element.name())?;

        xhtml.push_str(&format!("{}<{}", real_indent, tag_name));

        for (attr_name, attr_value) in element.attrs() {
            let attr_name = attr_name.trim();
            let attr_value = attr_value.trim();

            let (attr_name, attr_value) = match preprocess_attribute(attr_name, attr_value) {
                Some(pair) => pair,
                None => continue,
            };

            let is_html = is_likely_html(&attr_value);
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
                let escaped_attr_value = escape_xml(&attr_value);
                xhtml.push_str(&format!(" {}=\"{}\"", attr_name, escaped_attr_value));
            }
        }

        xhtml.push_str(">\n");

        for child in node.children() {
            walk(xhtml, child, indent + 1, extracted_docs);
        }

        xhtml.push_str(&format!("{}</{}>\n", real_indent, tag_name));
    }

    Some(())
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

    if let Some(dom) = to_dom(test_doc.data.clone()) {
        let element_count = count_element_nodes(dom.tree.root());
        // If we have more than just the auto-generated wrapper elements (html, head, body)
        // then this is likely real HTML content
        element_count > 3
    } else {
        false
    }
}

fn count_element_nodes(node: NodeRef<ScraperNode>) -> usize {
    let mut count = 0;

    match node.value() {
        ScraperNode::Element(_) => {
            count += 1;
            for child in node.children() {
                count += count_element_nodes(child);
            }
        }
        ScraperNode::Document => {
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
