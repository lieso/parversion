use xmltree::Element;
use std::io::Cursor;
use std::str::from_utf8;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::constants;

pub fn is_valid_xml(xml_string: &str) -> bool {
    match Element::parse(xml_string.as_bytes()) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn string_to_xml(data: &str) -> Option<String> {
    log::trace!("In string_to_xml");

    let mut xhtml = String::from("");

    let sanitized = data.replace("\n", "");

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut sanitized.as_bytes())
        .unwrap();

    walk(&mut xhtml, &dom.document, 0);

    if xhtml.trim().is_empty() {
        return None;
    }

    log::debug!("{}", xhtml);

    Some(xhtml)
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

pub fn preprocess_xml(xml_string: &str) -> String {
    let mut root = Element::parse(xml_string.as_bytes()).expect("Unable to parse XML");

    fn remove_attributes(element: &mut Element) {
        element.attributes.retain(|attr, _| {
            !constants::UNSEEN_BLACKLISTED_ATTRIBUTES.contains(&attr.as_str())
        });

        for child in &mut element.children {
            if let xmltree::XMLNode::Element(ref mut el) = child {
                remove_attributes(el);
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
    remove_attributes(&mut root);

    let mut buffer = Cursor::new(Vec::new());
    root.write(&mut buffer).expect("Could not write root");

    let buf = buffer.into_inner();
    let as_string = from_utf8(&buf).expect("Found invalid UTF-8");

    log::debug!("preprocessed_xml: {}", as_string);

    return as_string.to_string();
}
