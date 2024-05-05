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

impl Xml {
    pub fn parse(cursor: std::io::Cursor<T>) -> Result<Xml, Errors> {
        match Element::parse(&mut cursor) {
            Ok(element) => {
                let xml = Xml {
                    element: Some(element),
                    text: None,
                }
                
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

    pub fn get_children(&self) -> Vec<Xml> {
        if let Some(element) = self.element {
            return element.children.iter().filter_map(|child| {
                XMLNode::Element(child_element) => {
                    let xml = Xml {
                        element: Some(child_element),
                        text: None,
                    }

                    Some(xml)
                }
                XMLNode::Text(child_text) => {
                    let xml = Xml {
                        element: None,
                        text: Some(child_text),
                    }

                    Some(xml)
                }
                _ => None,

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
}
