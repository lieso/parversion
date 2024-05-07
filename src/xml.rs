use std::fmt;
use std::io::{Cursor, Read};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use xmltree::{Element, XMLNode};

use crate::error::{Errors};

#[derive(Clone, Debug)]
pub struct Xml {
    element: Option<Element>,
    text: Option<String>,
}

impl Serialize for Xml {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(element) = self.element {
            serializer.serialize_str(&element_to_string(&element))
        } else if let Some(text) = self.text {
            serializer.serialize_str(&text)
        } else {
            serializer.serialize_str("")
        }
    }
}

impl fmt::Display for Xml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(element) = self.element {
            write!(f, "{}", &element_to_string(&element))
        } else if let Some(text) = self.text {
            write!(f, "{}", &text)
        } else {
            write!(f, "{}", "")
        }
    }
}

impl Xml {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Xml, Errors> {
        match Element::parse(&mut reader) {
            Ok(element) => {
                let xml = Xml {
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

    pub fn without_children(&self) -> Xml {
        if self.element.is_some() {

            let mut copy = self.clone();

            copy.element.unwrap().children.clear();

            copy

        } else {
            self.clone()
        }
    }

    pub fn from_void() -> Xml {
        Xml {
            element: None,
            text: None,
        }
    }
}

impl Xml {
    pub fn get_element_tag_name(&self) -> String {
        if let Some(element) = self.element {
            return element.name.clone();
        }

        "".to_string()
    }

    pub fn get_attributes(&self) -> Vec<String> {
        if let Some(element) = self.element {
            return element.attributes.keys().cloned().collect();
        }

        Vec::new()
    }

    pub fn get_attribute_value(&self, name: &str) -> Option<String> {
        if self.element.is_none() {
            log::warn!("Attempting to get attribute: {} on Xml, but xml is not an element", name);
            return None;
        }

        Some(self.element.unwrap().attributes.get(name).cloned())
    }

    pub fn get_children(&self) -> Vec<Xml> {
        if let Some(element) = self.element {
            return element.children.iter().filter_map(|child| {
                match child {
                    XMLNode::Element(child_element) => {
                        let xml = Xml {
                            element: Some(child_element.clone()),
                            text: None,
                        };

                        Some(xml)
                    },
                    XMLNode::Text(child_text) => {
                        let xml = Xml {
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

    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.element.is_none()
    }

    pub fn to_string(&self) -> String {
        if let Some(element) = self.element {
            format!("{}", &element_to_string(&element))
        } else if let Some(text) = self.text {
            format!("{}", &text)
        } else {
            format!("{}", "")
        }
    }
}
