use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDataStructure {
    pub root_node_xpath: Option<String>,
    pub parent_node_xpath: Option<String>,
    pub next_item_xpath: Option<String>,
}
