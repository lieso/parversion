use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Classification {
    pub id: ID,
    pub lineage: Lineage,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub structure: String,
}
