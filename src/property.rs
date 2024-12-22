use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PropertyPath {
    pub segments: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Property {
    //pub name: String,
    pub property_type: String,
    pub description: String,
    pub path: Vec<PropertyPath>,
}
