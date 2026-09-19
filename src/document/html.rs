use ego_tree::{Tree, NodeId, NodeRef, NodeMut};
use scraper::{Html as ScraperHtml, Node as ScraperNode};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::{Arc, RwLock};
use xmltree::Element;
use rayon::prelude::*;

use crate::prelude::*;
use crate::context::Context;
use crate::data_node::DataNode;
use crate::meta_context::MetaContext;
use crate::document_node::{DocumentNode, DocumentNodeData};
use crate::graph_node::{Graph, GraphNode};
use crate::hash::Hash;
use crate::document::{Document, DocumentType, DocumentMetadata};

const MAX_SUBTREE_SIZE: usize = 1000;

pub struct Html;

impl Html {
    pub fn to_meta_context(
        metadata: &DocumentMetadata,
        data: String
    ) -> Result<(Vec<MetaContext>, Vec<Document>), Errors> {
        log::trace!("In to_meta_context");

        let (document_roots, other_documents) = Self::get_document_nodes(data)?;

        let meta_contexts = document_roots
            .into_par_iter()
            .map(|document_root| { 
                let document_root = Arc::new(RwLock::new(document_root.clone()));

                let contexts: Arc<RwLock<HashMap<ContextID, Arc<Context>>>> = Arc::new(RwLock::new(HashMap::new()));
                let contexts_lookup: Arc<RwLock<HashMap<ID, Arc<Context>>>> = Arc::new(RwLock::new(HashMap::new()));

                fn recurse(
                    document_node: Arc<RwLock<DocumentNode>>,
                    parent_lineage: &Lineage,
                    contexts: Arc<RwLock<HashMap<ContextID, Arc<Context>>>>,
                    contexts_lookup: Arc<RwLock<HashMap<ID, Arc<Context>>>>,
                    parents: Vec<Arc<RwLock<GraphNode>>>,
                ) -> Arc<RwLock<GraphNode>> {
                    let (hash, lineage, fields, description, network_name) = {
                        let lock = read_lock!(document_node);
                        let hash = lock.get_hash();
                        let lineage = parent_lineage.with_hash(hash.clone());
                        (hash, lineage, lock.get_fields(), lock.get_description(), lock.get_name())
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

                    let indexed_lineages = Arc::new(RwLock::new(HashMap::new()));

                    let context = Arc::new(Context {
                        id: ID::new(),
                        acyclic_lineage: data_node.lineage.acyclic(),
                        lineage: data_node.lineage.clone(),
                        indexed_lineages,
                        document_node: Arc::clone(&document_node),
                        graph_node: Arc::clone(&graph_node),
                        data_node: Arc::clone(&data_node),
                        network_name,
                    });

                    {
                        let mut lock = write_lock!(contexts);
                        lock.insert(context.id.clone(), Arc::clone(&context));
                    }

                    { 
                        let mut lock = write_lock!(contexts_lookup);
                        lock.insert(data_node.id.clone(), Arc::clone(&context));
                        lock.insert(read_lock!(document_node).id.clone(), Arc::clone(&context));
                        lock.insert(read_lock!(graph_node).id.clone(), Arc::clone(&context));
                    }

                    {
                        let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
                            .get_children()
                            .into_par_iter()
                            .map(|child| {
                                recurse(
                                    Arc::new(RwLock::new(child)),
                                    &data_node.lineage,
                                    Arc::clone(&contexts),
                                    Arc::clone(&contexts_lookup),
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

                let origin_hash = Hash::from_str(&metadata.origin.clone().unwrap_or_default());
                let initial_lineage = Lineage::new().with_hash(origin_hash);

                let graph_root = recurse(
                    Arc::clone(&document_root),
                    &initial_lineage,
                    contexts.clone(),
                    contexts_lookup.clone(),
                    Vec::new(),
                );

                let acyclic_subgraph_hash = {
                    let lock = read_lock!(graph_root);
                    lock.acyclic_subgraph_hash()
                };

                let contexts = read_lock!(contexts).clone();
                let contexts_lookup = read_lock!(contexts_lookup).clone();

                MetaContext {
                    contexts,
                    graph_root,
                    contexts_lookup,
                    document_type: DocumentType::Html,
                    acyclic_subgraph_hash,
                }
            })
            .collect();

        Ok((meta_contexts, other_documents))
    }

    pub fn from_meta_context(
        meta_context: &MetaContext,
        render_ids: Option<&HashSet<GraphNodeID>>,
    ) -> Result<String, Errors> {
        let graph_root = meta_context.graph_root.clone();

        let mut result: String = String::new();

        fn recurse(
            meta_context: &MetaContext,
            render_ids: Option<&HashSet<GraphNodeID>>,
            graph_node: Graph,
            result: &mut String,
        ) {
            let current_id = read_lock!(graph_node).id.clone();
            let current_context = meta_context.contexts_lookup.get(&current_id).unwrap();
            let document_node = &current_context.document_node;
            let children = read_lock!(graph_node).children.clone();

            let should_render = if let Some(render_ids) = render_ids {
                render_ids.contains(&current_id)
            } else {
                true
            };

            if should_render {
                let (a, _b) = read_lock!(document_node).to_string_components();
                result.push_str(&a);
            }

            for child in children {
                recurse(
                    meta_context,
                    render_ids.clone(),
                    Arc::clone(&child),
                    result,
                );
            }

            if should_render {
                let (_a, b) = read_lock!(document_node).to_string_components();
                result.push_str(b.as_deref().unwrap_or(""));
            }
        }

        recurse(
            meta_context,
            render_ids.clone(),
            Arc::clone(&graph_root),
            &mut result
        );

        Ok(result)
    }

    fn get_document_nodes(data: String) -> Result<(Vec<DocumentNode>, Vec<Document>), Errors> {
        if let Some(dom) = to_dom(data.clone()) {
            let _ = fs::create_dir("debug");

            let mut sizes = HashMap::new();
            calculate_subtree_sizes(dom.tree.root(), &mut sizes);

            let trees = cut(dom.tree.root(), &sizes);

            let result = trees
                .into_iter()
                .map(|tree: Tree<ScraperNode>| {
                    let mut xml = String::from("");

                    let mut other_documents: Vec<Document> = Vec::new();

                    walk(&mut xml, tree.root(), 0, &mut other_documents);

                    let reader = std::io::Cursor::new(xml);

                    let document_node = match Element::parse(reader) {
                        Ok(element) => Ok(
                            DocumentNode::new(
                                DocumentNodeData::Xml(
                                    xmltree::XMLNode::Element(element)
                                )
                            )
                        ),
                        Err(e) => {
                            log::error!("Could not parse XML: {}", e);

                            Err(Errors::XmlParseError)
                        }
                    }?;

                    Ok((document_node, other_documents))
                })
                .collect::<Result<Vec<_>, Errors>>()?
                .into_iter()
                .fold((Vec::new(), Vec::new()), |(mut acc), (document_node, other_documents)| {
                    acc.0.push(document_node);
                    acc.1.extend(other_documents);
                    acc
                });

            Ok(result)
        } else {
            unimplemented!()
        }
    }
}

fn to_dom(data: String) -> Option<ScraperHtml> {
    let sanitized = data.replace("\n", "");
    Some(ScraperHtml::parse_document(&sanitized))
}

fn find_cuts(
    node: NodeRef<ScraperNode>,
    sizes: &HashMap<NodeId, usize>,
    cuts: &mut HashSet<NodeId>
) -> bool {
    let mut already_cut = false;

    for child in node.children() {
        if find_cuts(child, sizes, cuts) {
            already_cut = true;
        }
    }

    if already_cut {
        return true;
    }

    if sizes[&node.id()] > MAX_SUBTREE_SIZE {
        cuts.insert(node.id());
        return true;
    }

    false
}

fn clone_full_subtree(node: NodeRef<ScraperNode>, mut dest: NodeMut<ScraperNode>) {
    for child in node.children() {
        let dest_child = dest.append(child.value().clone());
        clone_full_subtree(child, dest_child);
    }
}

fn clone_into(node: NodeRef<ScraperNode>, mut dest: NodeMut<ScraperNode>, cuts: &HashSet<NodeId>) {
    for child in node.children() {
        let mut dest_child = dest.append(child.value().clone());
        if !cuts.contains(&child.id()) {
            clone_into(child, dest_child, cuts);
        }
    }
}

fn build_cut_tree(cut_node: NodeRef<ScraperNode>) -> Tree<ScraperNode> {
    let mut chain: Vec<NodeRef<ScraperNode>> = cut_node.ancestors().collect();
    chain.reverse();
    chain.push(cut_node);

    let mut iter = chain.into_iter();
    let root_node = iter.next().unwrap();

    let mut new_tree = Tree::new(root_node.value().clone());
    let mut cursor_id = new_tree.root().id();

    let mut last = root_node;
    for ancestor in iter {
        cursor_id = new_tree
            .get_mut(cursor_id)
            .unwrap()
            .append(ancestor.value().clone())
            .id();
        last = ancestor;
    }

    let dest = new_tree.get_mut(cursor_id).unwrap();
    clone_full_subtree(last, dest);

    new_tree
}

fn cut<'a>(
    tree: NodeRef<'a, ScraperNode>,
    sizes: &HashMap<NodeId, usize>
) -> Vec<Tree<ScraperNode>> {

    let mut cuts: HashSet<NodeId> = HashSet::new();

    find_cuts(
        tree,
        sizes,
        &mut cuts
    );

    log::info!("Found {} cut(s) to make", cuts.len());

    let arena = tree.tree();
    let subtrees: Vec<Tree<ScraperNode>> = cuts
        .iter()
        .map(|node_id| {
            let node = arena.get(*node_id).expect("cut id must exist in tree");
            build_cut_tree(node)
        })
        .collect();

    let mut new_tree = Tree::new(tree.value().clone());
    clone_into(tree, new_tree.root_mut(), &cuts);

    let mut result = subtrees;
    result.push(new_tree);

    result
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
        "svg" | "script" | "iframe" | "input" | "button" | "link" | "meta" | "style" | "noscript" => None,
        _ => Some(tag_name.to_string()),
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

        let mut has_attributes = false;
        let mut attributes_str = String::new();

        for (attr_name, attr_value) in element.attrs() {
            let attr_name = attr_name.trim().to_string();
            let attr_value = attr_value.trim().to_string();

            has_attributes = true;

            let is_html = is_likely_html(&attr_value);
            let _is_javascript = false; // TODO: Check if attr_value is valid JavaScript

            if is_html {
                let html_doc = Document {
                    document_type: DocumentType::Html,
                    data: attr_value.to_string(),
                    metadata: DocumentMetadata {
                        origin: None,
                        date: None,
                        name: None,
                        description: None,
                        semantic_content_types: None,
                    },
                };
                extracted_docs.push(html_doc);
            }

            if _is_javascript {
                // TODO: Parse as JavaScript and create Document
            }

            if !is_html && !_is_javascript {
                let escaped_attr_value = escape_xml(&attr_value);
                attributes_str.push_str(&format!(" {}=\"{}\"", attr_name, escaped_attr_value));
            }
        }

        xhtml.push_str(&format!("{}<{}{}", real_indent, tag_name, attributes_str));
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
            name: None,
            description: None,
            semantic_content_types: None,
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

fn calculate_subtree_sizes(
    node: NodeRef<ScraperNode>,
    sizes: &mut HashMap<NodeId, usize>
) -> usize {
    let mut count = match node.value() {
        ScraperNode::Element(_) => 1,
        _ => 0,
    };

    for child in node.children() {
        count += calculate_subtree_sizes(child, sizes);
    }

    sizes.insert(node.id(), count);
    count
}
