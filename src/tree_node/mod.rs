use serde::{Serialize, Deserialize};
use std::sync::{Arc, Weak};
use uuid::Uuid;
use std::cell::RefCell;

use crate::xml::{Xml};
use crate::xml;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    pub hash: String,
    pub xml: Xml,
    pub parent: Option<Arc<Weak<TreeNode>>>,
    pub children: Vec<Arc<TreeNode>>,
}

pub fn build_tree(xml: String) -> Arc<TreeNode> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = Xml::parse(&mut reader).expect("Could not parse XML");

    TreeNodeBuilder::from_xml(&xml, None).finalize()
}

pub fn deep_copy(tree_node: &Arc<TreeNode>) -> Arc<TreeNode> {
    unimplemented!()
}

impl TreeNode {
    fn from_xml(xml: &Xml, parent: Option<Arc<RefCell<TreeNode>>>) -> Arc<Self> {
        TreeNodeBuilder::from_xml(xml, parent.map(|weak_parent| weak_parent.upgrade().unwrap())).finalize()
    }
}

#[derive(Clone, Debug)]
struct TreeNodeBuilder {
    pub id: String,
    pub hash: String,
    pub xml: Xml,
    pub parent: Option<Arc<Weak<RefCell<TreeNodeBuilder>>>>,
    pub children: Vec<Arc<RefCell<TreeNodeBuilder>>>,
}

impl TreeNodeBuilder {
    fn from_xml(xml: &Xml, parent: Option<Arc<RefCell<Self>>>) -> Arc<RefCell<Self>> {
        let node = Arc::new(RefCell::new(TreeNodeBuilder {
            id: Uuid::new_v4().to_string(),
            hash: xml::xml_to_hash(&xml),
            xml: xml.without_children(),
            parent: parent.as_ref().map(Arc::downgrade),
            children: Vec::new(),
        }));

        let weak_node = Arc::downgrade(&node);

        let children: Vec<Arc<RefCell<TreeNodeBuilder>>> = xml
            .get_children()
            .iter()
            .map(|child| TreeNodeBuilder::from_xml(child, Some(Weak::clone(&weak_node).upgrade().unwrap())))
            .collect();

        node.borrow_mut().children.extend(children);

        node
    }
}
