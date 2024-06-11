use serde::{Serialize, Deserialize};
use fancy_regex::Regex;

use crate::xml::{Xml};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataValue {
    //pub is_url: bool,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub regex: String,
    pub name: String,
    pub is_id: bool,
    pub is_url: bool,
    pub is_decorative: bool,
    pub is_js: bool,
    pub value: Option<NodeDataValue>,
}

impl NodeData {
    pub fn select(&self, xml: Xml) -> Option<NodeDataValue> {
        if let Ok(regex) = Regex::new(&self.regex) {
            log::debug!("Regex is ok");
            log::debug!("regex: {}", regex);

            let xml_string = xml.to_string();
            log::debug!("xml_string: {}", xml_string);

            let matches: Vec<&str> = regex
                .captures_iter(&xml_string)
                .filter_map(|cap| {
                    cap.expect("Could not capture").get(1).map(|mat| mat.as_str())
                })
                .collect();
            log::debug!("{:?}", matches);

            if let Some(first_match) = matches.first() {
                log::debug!("first_match: {}", first_match.to_string());
                return Some(NodeDataValue {
                    text: first_match.to_string()
                });
            }
        }

        Some(NodeDataValue {
            text: xml.to_string(),
        })
    }
}
