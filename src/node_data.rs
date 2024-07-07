use serde::{Serialize, Deserialize};

use crate::xml::{Xml};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ElementNodeMetadata {
    pub attribute: String,
    pub is_id: bool,
    pub is_url: bool,
    pub is_page_link: bool,
    pub is_action_link: bool,
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextNodeMetadata {
    pub is_informational: bool,
    pub is_primary_content: bool,
    pub is_main_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub element_fields: Option<ElementNodeMetadata>,
    pub text_fields: Option<TextNodeMetadata>,
    pub name: String,
}

impl NodeData {
    pub fn value(&self, xml: &Xml) -> String {
        if let Some(_text_fields) = &self.text_fields {
            let value = xml.to_string();
            return String::from(value.trim_matches(|c| c == ' ' || c == '\n'));
        }

        if let Some(element_fields) = &self.element_fields {
            let value = xml.get_attribute_value(&element_fields.attribute).unwrap();
            return String::from(value.trim_matches(|c| c == ' ' || c == '\n'));
        }

        panic!("NodeData neither has element or text fields!");
    }
}
