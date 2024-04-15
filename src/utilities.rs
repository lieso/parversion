use html5ever::driver::ParseOpts;
use markup5ever_rcdom as rcdom;
use html5ever::{parse_document};
use html5ever::tendril::TendrilSink;
use std::io;
use std::default::Default;
use std::string::String;
use xmltree::Element;
use std::io::Cursor;
use std::str::from_utf8;
use sled::{Db};
use bincode::{serialize, deserialize};
use std::error::Error;
use xpath_reader::Reader;

use crate::models::*;

const BLACKLISTED_ATTTRIBUTES: [&str; 7] = [
    "style", "bgcolor", "border", "cellpadding", "cellspacing",
    "width", "height", 
];

pub fn is_valid_xml(xml_string: &str) -> bool {
    match Element::parse(xml_string.as_bytes()) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn is_valid_html(html_string: &str) -> bool {
    let parser = parse_document(rcdom::RcDom::default(), ParseOpts::default());

    let dom = parser.one(html_string);
    log::debug!("dom.errors: {:?}", dom.errors);

    if !dom.errors.is_empty() {
        for error in &dom.errors {
            log::debug!("Error: {}", error);
         }
    }

    dom.errors.is_empty()
}

pub fn html_to_xhtml(html: &str) -> io::Result<String> {
    let xhtml = remove_doctype(&html);
    log::warn!("NOT IMPLEMENTED");
    Ok(xhtml)
}

pub fn remove_doctype(html: &str) -> String {
    let doctype_pattern = regex::Regex::new(r"(?i)<!DOCTYPE\s+[^>]*>").unwrap();
    doctype_pattern.replace(html, "").to_string()
}

pub fn preprocess_xml(xml_string: &str) -> String {
    let mut root = Element::parse(xml_string.as_bytes()).expect("Unable to parse XML");

    fn remove_attributes(element: &mut Element) {
        element.attributes.retain(|attr, _| !BLACKLISTED_ATTTRIBUTES.contains(&attr.as_str()));

        for child in &mut element.children {
            if let xmltree::XMLNode::Element(ref mut el) = child {
                remove_attributes(el);
            }
        }
    }

    remove_attributes(&mut root);

    let mut buffer = Cursor::new(Vec::new());
    root.write(&mut buffer).expect("Could not write root");

    let buf = buffer.into_inner();
    let as_string = from_utf8(&buf).expect("Found invalid UTF-8");

    return as_string.to_string();
}

pub fn get_element_xml(element: &Element) -> String {
    let mut opening_tag = format!("<{}", element.name);

    for (attr_key, attr_value) in element.attributes.iter() {
        opening_tag.push_str(&format!(" {}=\"{}\"", attr_key, attr_value));
    }

    opening_tag.push('>');

    let closing_tag = format!("</{}>", element.name);

    format!("{}{}", opening_tag, closing_tag)
}

