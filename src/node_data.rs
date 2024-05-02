use serde::{Serialize, Deserialize};

use crate::utility;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataValue {
    //pub is_url: bool,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub attribute: String,
    pub name: String,
    pub regex: String,
    pub value: Option<NodeDataValue>,
}

impl NodeData {
    pub fn select(&self, xml: String) -> Option<NodeDataValue> {
        if let Some(xpath) = &self.xpath {
            if let Ok(result) = utility::apply_xpath(&xml, &xpath) {
                Some(NodeDataValue {
                    text: result,
                })
            } else {
                log::warn!("Unable to apply xpath: {} to xml: {}", xpath, xml);
                None
            }
        } else {
            Some(NodeDataValue {
                text: xml,
            })
        }
    }
}
