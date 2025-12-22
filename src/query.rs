use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Query {
    pub id: ID,
    pub hash: Hash,
}

impl Query {
    pub fn to_string(&self) -> String {
        unimplemented!()
    }
}
