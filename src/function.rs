use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Function {
    pub id: ID,
    pub hash: Hash,
    pub code: String,
}
