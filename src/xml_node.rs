use std::fmt;
use std::io::{Read, Write};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use xmltree::{Element, XMLNode};
use serde::de::{self, Visitor};
use sha2::{Sha256, Digest};
use pathetic;
use url::Url;
use std::collections::{HashMap};

use crate::error::{Errors};
use crate::constants;
use crate::graph_node::{GraphNodeData};

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
}

impl XmlNode {
    pub fn get_element_tag_name(&self) -> String {
        if let Some(element) = &self.element {
            return element.name.clone();
        }

        "".to_string()
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

    pub fn get_opening_tag(&self) -> String {
        let element = self.element.clone().unwrap();
        let mut tag = format!("<{}", element.name);

        // We sort attributes here to ensure any hashes we calculate on HTML snippets are deterministic
        let mut attributes: Vec<(&String, &String)> = element.attributes.iter().collect();
        attributes.sort_by(|a, b| a.0.cmp(b.0));

        for (attr, value) in attributes {
            tag.push_str(&format!(" {}=\"{}\"", attr, value.replace("\"", "&quot;")));
        }
        tag.push('>');

        tag
    }

    pub fn get_closing_tag(&self) -> String {
        let element = self.element.clone().unwrap();
        format!("</{}>", element.name)
    }
}

fn element_to_string(element: &Element) -> String {
    let mut output = Vec::new();

    fn write_element<W: Write>(element: &Element, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "<{}", element.name)?;

        if element.children.is_empty() { 
            write!(writer, "/>")?;
        } else {
            write!(writer, ">")?;

            for child in &element.children {
                if let XMLNode::Element(child_element) = child {
                    write_element(child_element, writer)?;
                }

                if let XMLNode::Text(child_text) = child {
                    write!(writer, "{}", child_text)?;
                }
            }

            write!(writer, "</{}>", element.name)?;
        }

        Ok(())
    }

    write_element(&element, &mut output).unwrap();

    String::from_utf8(output).unwrap()
}
