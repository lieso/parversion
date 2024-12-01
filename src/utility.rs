use xmltree::Element;
use std::io::{Write, Cursor};
use std::fs::File;
use std::str::from_utf8;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use url::Url;

use crate::constants;
use crate::environment;
use crate::error::{Errors};

pub fn remove_duplicate_sequences(vec: Vec<String>) -> Vec<String> {
    if vec.is_empty() {
        return vec;
    }

    let mut result = Vec::new();
    let mut iter = vec.into_iter().peekable();

    while let Some(current) = iter.next() {
        result.push(current.clone());

        while let Some(next) = iter.peek() {
            if next == &current {
                iter.next();
            } else {
                break;
            }
        }
    }

    result
}

pub fn is_valid_xml(xml_string: &str) -> bool {
    match Element::parse(xml_string.as_bytes()) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn string_to_xml(data: &str) -> Result<String, Errors> {
    log::trace!("In string_to_xml");

    let mut xhtml = String::from("");

    let sanitized = data.replace("\n", "");

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

pub fn preprocess_xml(url: Option<&str>, xml_string: &str) -> String {
    let mut root = Element::parse(xml_string.as_bytes()).expect("Unable to parse XML");

    fn remove_attributes(url: Option<&str>, element: &mut Element) {
        element.attributes.retain(|attr, value| {

            // TODO: should we remove origin URLs from a web document?

            // We never need to know that a web document links to itself
            // So we remove any instances of URLs that match the URL the document came from.
            let is_self_link = {
                if attr == "href" {
                    if let Some(url) = url {
                        let (origin, path) = get_origin_and_path(url).unwrap();

                        if is_relative_url(value) {
                            *value == path
                        } else {
                            value == url
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            !constants::UNSEEN_BLACKLISTED_ATTRIBUTES.contains(&attr.as_str()) &&
            value.as_str().len() < 500 &&
            !is_self_link
        });

        for child in &mut element.children {
            if let xmltree::XMLNode::Element(ref mut el) = child {
                remove_attributes(url, el);
            }
        }
    }

    fn remove_elements(element: &mut Element) {
        element.children.retain(|child| {
            if let xmltree::XMLNode::Element(ref el) = child {
                !constants::UNSEEN_BLACKLISTED_ELEMENTS.contains(&el.name.as_str())
            } else {
                true
            }
        });

        for child in &mut element.children {
            if let xmltree::XMLNode::Element(ref mut el) = child {
                remove_elements(el);
            }
        }
    }

    remove_elements(&mut root);
    remove_attributes(url, &mut root);

    let mut buffer = Cursor::new(Vec::new());
    root.write(&mut buffer).expect("Could not write root");

    let buf = buffer.into_inner();
    let as_string = from_utf8(&buf).expect("Found invalid UTF-8").to_string();

    if environment::is_local() {
        let mut file = File::create("./debug/unprocessed.xml").expect("Could not create file");
        file.write_all(xml_string.as_bytes()).expect("Could not write to file");

        let mut file = File::create("./debug/preprocessed.xml").expect("Could not create file");
        file.write_all(as_string.as_bytes()).expect("Could not write to file");
    }

    return as_string
}

fn get_origin_and_path(url_str: &str) -> Option<(String, String)> {
    if let Ok(parsed_url) = Url::parse(url_str) {
        let origin = parsed_url.origin().unicode_serialization();
        let path = parsed_url.path().trim_start_matches('/').to_string();

        Some((origin, path))
    } else {
        None
    }
}

fn is_relative_url(url: &str) -> bool {
    !Url::parse(url).is_ok()
}
