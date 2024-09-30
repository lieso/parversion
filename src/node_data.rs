use serde::{Serialize, Deserialize};

use crate::xml_node::{XmlNode};
use crate::content::{ContentValue, ContentValueMetadata};
use crate::graph_node::{Graph};
use crate::macros::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ElementData {
    pub attribute: String,
    pub is_page_link: bool,
    pub is_peripheral_content: bool,
    pub is_advertisement: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextData {
    pub is_presentational: bool,
    pub is_title: bool,
    pub is_primary_content: bool,
    pub is_peripheral_content: bool,
    pub is_advertisement: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub element: Option<ElementData>,
    pub text: Option<TextData>,
    pub name: String,
}

pub fn apply_data(
    node_data: NodeData,
    output_node: Graph<XmlNode>,
) -> Option<ContentValue> {
    log::trace!("In apply_data");

    // Discarding text nodes interpreted to be presentational, not informative
    if let Some(text_data) = &node_data.text {
        if text_data.is_presentational {
            log::info!("Discarding presentational text node data");
            return None;
        }
    }

    // Discarding href values that mutate instead of linking to others
    if let Some(element_data) = &node_data.element {
        if element_data.attribute == "href" {
            if !element_data.is_page_link {
                log::info!("Discarding href action link...");
                return None;
            }
        }
    }

    // Discarding advertisments
    let is_advertisement = {
        node_data.clone().text.map_or(false, |text| text.is_advertisement) ||
        node_data.clone().element.map_or(false, |element| element.is_advertisement)
    };
    if is_advertisement {
        log::info!("Discarding advertisement");
        return None;
    }

    let output_node_xml: XmlNode = read_lock!(output_node).data.clone();

    let content_value = ContentValue {
        name: node_data.name.clone(),
        value: node_data.value(&output_node_xml),
        meta: ContentValueMetadata {
            is_title: node_data.text.clone().map_or(false, |text| text.is_title),
            is_primary_content: node_data.text.clone().map_or(false, |text| text.is_primary_content),
            is_url: node_data.element.clone().map_or(false, |element| {
                element.attribute == "href"
            })
        },
    };

    Some(content_value)
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
