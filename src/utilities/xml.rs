extern crate xml;

use xml::reader::{EventReader, XmlEvent};
use xml::ParserConfig;

pub fn is_valid_xml(xml_content: &str) -> bool {
    let config = ParserConfig::new().trim_whitespace(true);
    let parser = EventReader::new_with_config(xml_content.as_bytes(), config);

    for e in parser {
        match e {
            Ok(XmlEvent::EndDocument) => return true,
            Err(_) => return false,
            _ => continue,
        }
    }

    false
}
