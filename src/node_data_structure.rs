use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataStructure {
    pub recursive_attribute: Option<String>,
    pub root_node_attribute_values: Option<Vec<String>>,
    pub parent_node_attribute_value: Option<String>,
    pub next_item_xpath: Option<String>,
}
