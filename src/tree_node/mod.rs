use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    pub hash: String,
    pub xml: Xml,
    pub parent: Option<Arc<Node>>,
    pub children: Vec<Arc<Node>>,
}
