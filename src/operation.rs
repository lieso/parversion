use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::mutation::Mutation;
use crate::query::Query;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Operation {
    pub id: ID,
    pub hash: Hash,
    pub query: Option<Query>,
    pub mutation: Option<Mutation>,
}

impl Operation {
    pub fn new(hash: &Hash) -> Self {
        Operation {
            id: ID::new(),
            hash: hash.clone(),
            query: None,
            mutation: None,
        }
    }

    pub fn is_no_op(&self) -> bool {
        self.query.is_none() && self.mutation.is_none()
    }
}
