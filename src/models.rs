use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub xpath: String,
    pub name: String,
    pub is_url: bool,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub id: String,
    pub parent: Weak<Node>,
    pub subtree_hash: String,
    pub ancestry_hash: String,
    pub xml: String,
    pub xml_hash: String,
    pub tag: String,
    pub interpret: bool,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexObject {
    pub id: String,
    pub type_id: String,
    pub values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Relationship {
    pub id: String,
    pub complex_type_id: String,
    pub origin_field: String,
    pub target_field: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub basis_nodes: Node,
    pub complex_types: HashMap<String, Vec<ComplexType>>,
    pub complex_objects: HashMap<String, Vec<ComplexObject>>,
    pub lists: HashMap<String, Vec<String>>,
    pub relationships: HashMap<String, Relationship>,
}
