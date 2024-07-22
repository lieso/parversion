use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNode {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<Mutex<BasisNode>>>,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}
