use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lineage {
    value: String,
}

impl Lineage {
    pub fn from_hashes(hashes: Vec<Hash>) -> Self {
        unimplemented!()
    }
}
