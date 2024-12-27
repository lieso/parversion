use serde::{Serialize, Deserialize};
use std::collections::{HashSet};

use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentProfile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<u64>,
}
