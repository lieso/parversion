extern crate xml;
extern crate xmltree;

use xmltree::Element;
use std::io::Cursor;
use std::str::from_utf8;

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
