use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub struct NodeData {
    pub xpath: String,
    pub key: String,
    pub is_url: bool,
    pub value: Option<String>,
}

pub struct Node {
    pub id: String,
    pub parent: Weak<Node>,
    pub hash: RefCell<Option<String>>,
    pub xml: String,
    pub tag: String,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}
