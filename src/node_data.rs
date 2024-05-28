use serde::{Serialize, Deserialize};

use crate::xml::{Xml};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataValue {
    //pub is_url: bool,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub attribute: Option<String>,
    pub name: String,
    pub regex: String,
    pub is_id: bool,
    pub is_url: bool,
    pub is_decorative: bool,
    pub value: Option<NodeDataValue>,
}

impl NodeData {
    pub fn select(&self, xml: Xml) -> Option<NodeDataValue> {

        if let Some(attribute) = &self.attribute {
            return Some(NodeDataValue {
                text: xml.get_attribute_value(attribute).unwrap()
            });
        }

        Some(NodeDataValue {
            text: xml.to_string(),
        })
    }
}
