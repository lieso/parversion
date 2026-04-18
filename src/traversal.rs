use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::xpath::XPath;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TraversalValue {
    pub name: String,
    pub xpath: XPath,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub candidate: XPath,
    pub parent_values: Vec<TraversalValue>,
    pub candidate_values: Vec<TraversalValue>,
    pub filter_function: String,
}
