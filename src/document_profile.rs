use serde::{Serialize, Deserialize};
use std::collections::{HashSet};

use crate::prelude::*;
use crate::transformation::DocumentTransformation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentProfile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<u64>,
    pub transformations: Vec<DocumentTransformation>,
}
