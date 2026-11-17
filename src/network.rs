use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Network {
    pub id: ID,
    pub name: String,
    pub description: Option<String>,
}
