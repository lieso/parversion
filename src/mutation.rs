use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mutation {
    pub id: ID,
    pub hash: Hash,
}

impl Mutation {
    pub fn to_string(&self) -> String {
        unimplemented!()
    }
}
