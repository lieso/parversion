use serde::{Serialize, Deserialize};

use crate::xml_node::{XmlNode};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ElementData {
    pub attribute: String,
    pub is_page_link: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextData {
    pub is_presentational: bool,
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub element: Option<ElementData>,
    pub text: Option<TextData>,
    pub name: String,
}

impl NodeData {
    pub fn value(&self, xml: &XmlNode) -> String {
        if let Some(_text) = &self.text {
            let value = xml.to_string();
            return String::from(value.trim_matches(|c| c == ' ' || c == '\n'));
        }

        if let Some(element) = &self.element {
            let value = xml.get_attribute_value(&element.attribute).unwrap();
            return String::from(value.trim_matches(|c| c == ' ' || c == '\n'));
        }

        panic!("NodeData neither has element or text fields!");
    }
}
