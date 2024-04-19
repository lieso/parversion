use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::{Rc};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
}

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
    pub hash: String,
    pub parent: RefCell<Option<Rc<Node>>>,
    pub xml: String,
    pub tag: String,
    pub interpret: bool,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexType {
    pub id: String,
    pub values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DerivedType {
    pub id: String,
    pub complex_mapping: HashMap<String, HashMap<String, String>>,
    pub values: HashMap<String, String>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Rc<Node>,
    pub basis_tree: Option<Rc<Node>>,
    pub primitives: Vec<HashMap<String, String>>,
    pub complex_types: Vec<ComplexType>,
    pub complex_objects: Vec<ComplexObject>,
    pub lists: Vec<String>,
    pub relationships: Vec<Relationship>,
}
