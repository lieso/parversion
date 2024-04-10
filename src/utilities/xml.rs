extern crate xml;
extern crate xmltree;

use xmltree::Element;

const BLACKLISTED_ATTTRIBUTES: [&str; 6] = [
    "style", "bgColor", "border", "cellpadding", "cellspacing", "width"
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

    return root.get_text().unwrap().into_owned();
}
