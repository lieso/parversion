use std::fmt;
use std::io::{Cursor, Read};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use xmltree::{Element, XMLNode};
use xmltree::EmitterConfig;
use serde::de::{self, Visitor};
use sha2::{Sha256, Digest};
use pathetic;
use url::Url;

use crate::error::{Errors};
use crate::constants;
use crate::graph_node::{GraphNodeData};

#[derive(Clone, Debug)]
pub struct XmlNode {
    pub element: Option<Element>,
    pub text: Option<String>,
}

impl GraphNodeData for XmlNode {
    fn new(_description: String) -> Self {
        XmlNode {
            element: None,
            text: None,
        }
    }

    fn describe(&self) -> String {
        self.to_string()
    }
}

pub fn xml_to_hash(xml: &XmlNode) -> String {
    if xml.is_text() {
        return constants::TEXT_NODE_HASH.to_string();
    }

    let mut hasher = Sha256::new();

    let mut hasher_items = Vec::new();
    hasher_items.push("TAG:".to_owned() + &xml.get_element_tag_name());

    for (attribute, value) in xml.element.clone().unwrap().attributes {
        hasher_items.push("ATTRIBUTE:".to_owned() + &attribute.clone());

        if attribute == "href" {
            let parts = url_to_hash_parts(&value);
            for part in parts {
                hasher_items.push("HREF:".to_owned() + &part);
            }
        }

        if attribute == "class" {
            for class in value.split_whitespace() {
                //hasher_items.push("CLASS:".to_owned() + &class);
                hasher_items.push("CLASS".to_owned());
            }
        }
    }

    hasher_items.sort();

    hasher.update(hasher_items.join(""));

    format!("{:x}", hasher.finalize())
}

impl Serialize for XmlNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct XmlNodeVisitor;

impl<'de> Visitor<'de> for XmlNodeVisitor {
    type Value = XmlNode;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("valid XML in a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Ok(element) = Element::parse(value.as_bytes()) {
            Ok(XmlNode {
                element: Some(element),
                text: None,
            })
        } else {
            Ok(XmlNode {
                element: None,
                text: Some(value.to_owned()),
            })
        }
    }
}

impl<'de> Deserialize<'de> for XmlNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(XmlNodeVisitor)
    }
}

impl fmt::Display for XmlNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_string())
    }
}

impl XmlNode {
    pub fn parse<R: Read>(reader: &mut R) -> Result<XmlNode, Errors> {
        match Element::parse(reader) {
            Ok(element) => {
                let xml = XmlNode {
                    element: Some(element),
                    text: None,
                };
                
                Ok(xml)
            }
            _ => {
                Err(Errors::XmlParseError)
            }
        }
    }

    pub fn without_children(&self) -> XmlNode {
        if self.element.is_some() {

            let mut copy = self.clone();

            copy.element.as_mut().unwrap().children.clear();

            copy

        } else {
            self.clone()
        }
    }

    pub fn from_void() -> XmlNode {
        XmlNode {
            element: None,
            text: None,
        }
    }
}

impl XmlNode {
    pub fn get_element_tag_name(&self) -> String {
        if let Some(element) = &self.element {
            return element.name.clone();
        }

        "".to_string()
    }

    pub fn get_attributes(&self) -> Vec<String> {
        if let Some(element) = &self.element {
            return element.attributes.keys().cloned().collect();
        }

        Vec::new()
    }

    pub fn get_attribute_value(&self, name: &str) -> Option<String> {
        if self.element.is_none() {
            log::warn!("Attempting to get attribute: {} on XmlNode, but xml is not an element", name);
            return None;
        }

        self.element.clone().unwrap().attributes.get(name).cloned()
    }

    pub fn get_children(&self) -> Vec<XmlNode> {
        if let Some(element) = &self.element {
            return element.children.iter().filter_map(|child| {
                match child {
                    XMLNode::Element(child_element) => {
                        let xml = XmlNode {
                            element: Some(child_element.clone()),
                            text: None,
                        };

                        Some(xml)
                    },
                    XMLNode::Text(child_text) => {
                        let xml = XmlNode {
                            element: None,
                            text: Some(child_text.to_string()),
                        };

                        Some(xml)
                    },
                    _ => None,
                }

            }).collect();
        }

        Vec::new()
    }

    pub fn is_text(&self) -> bool {
        self.text.is_some()
    }

    pub fn is_element(&self) -> bool {
        self.element.is_some()
    }

    pub fn to_string(&self) -> String {
        if let Some(element) = &self.element {
            format!("{}", &element_to_string(&element))
        } else if let Some(text) = &self.text {
            format!("{}", &text.trim())
        } else {
            format!("{}", "")
        }
    }

    pub fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.to_string());

        format!("{:x}", hasher.finalize())
    }

    pub fn _is_equal(&self, xml: XmlNode) -> bool {
        if let Some(element_a) = &self.element {
            if let Some(element_b) = xml.element {
                if element_a.name != element_b.name {
                    return false;
                }

                let attributes_a = &element_a.attributes;
                let attributes_b = &element_b.attributes;

                if attributes_a.keys().len() != attributes_b.keys().len() {
                    return false;
                }

                for (attribute, value_a) in attributes_a {
                    let value_b = attributes_b.get(attribute);

                    if let Some(value_b) = value_b {
                        if value_a != value_b {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

            } else {
                return false;
            }

        } else if let Some(text_a) = &self.text {
            if let Some(text_b) = xml.text {
                return text_a == &text_b;
            } else {
                return false;
            }
        }

        true
    }
}

fn element_to_string(element: &Element) -> String {
     let mut config = EmitterConfig::new();
     config.write_document_declaration = false;

     let mut cursor = Cursor::new(Vec::new());

     element.write_with_config(&mut cursor, config).unwrap();

     let serialized_xml = String::from_utf8(cursor.into_inner()).unwrap();

     // TODO
     let serialized_xml = serialized_xml.replace(" xmlns=\"http://www.w3.org/1999/xhtml\"", "");

     serialized_xml
}

pub fn get_opening_tag(element: &Element) -> String {
    let mut tag = format!("<{}", element.name);

    for (attr, value) in &element.attributes {
        tag.push_str(&format!(" {}=\"{}\"", attr, value.replace("\"", "&quot;")));
    }
    tag.push('>');

    tag
}

pub fn get_closing_tag(element: &Element) -> String {
    format!("</{}>", element.name)
}

fn url_to_hash_parts(url: &str) -> Vec<String> {
    // This is how I'm checking if a URL is absolute...
    // pathetic is a library for dealing with relative URLs
    if Url::parse(url).is_ok() {
        return Vec::new();
    }

    let parsed_url = pathetic::Uri::new(url).expect("Failed to parse the URL");

    let mut output = Vec::new();

    let path = parsed_url.path_segments().collect::<Vec<_>>().join("/");
    output.push(path);

    if let Some(query) = parsed_url.query() {
        for param in query.split('&') {
            if let Some((name, _)) = param.split_once('=') {
                output.push(name.to_string());
            } else {
                output.push(param.to_string());
            }
        }
    }

    output
}
