use std::sync::{Arc};

use crate::xml::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    pub hash: String,
    pub xml: Xml,
    pub parent: Option<Arc<Node>>,
    pub children: Vec<Arc<Node>>,
}

pub fn build_tree(xml: String) -> Arc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    TreeNode::from_xml(&xml, None)
}
