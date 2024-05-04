use serde::{Serialize, Deserialize};
use std::io::Cursor;
use xmltree::Element;

use crate::utility;

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
    pub value: Option<NodeDataValue>,
}

impl NodeData {
    pub fn select(&self, xml: String) -> Option<NodeDataValue> {

        // TODO: apply regex

        if let Some(attribute) = &self.attribute {
            let cursor = Cursor::new(xml.as_bytes());
            let element = Element::parse(cursor).expect("Could not parse XML string");
            let value = element.attributes.get(attribute).unwrap();


            Some(NodeDataValue {
                text: value.to_string(),
            })
        } else {

            Some(NodeDataValue {
                text: xml.to_string(),
            })

        }
    }
}
