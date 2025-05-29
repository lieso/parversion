use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    id: ID,
    name: String,
    description: String,
}
