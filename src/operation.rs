use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::mutation::Mutation;
use crate::query::Query;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum QueryOrMutation {
    Query(Query),
    Mutation(Mutation),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Operation {
    operation: QueryOrMutation,
}
