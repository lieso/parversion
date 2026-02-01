use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathSegment {
    pub id: ID,
    #[serde(flatten)]
    pub kind: PathSegmentKind,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "value")]
pub enum PathSegmentKind {
    Key(String),
    Index(usize),
    VariableIndex(char),
}

impl PathSegment {
    pub fn new_key_segment(key: String) -> Self {
        PathSegment {
            id: ID::new(),
            kind: PathSegmentKind::Key(key),
        }
    }

    pub fn new_index_segment(index: usize) -> Self {
        PathSegment {
            id: ID::new(),
            kind: PathSegmentKind::Index(index),
        }
    }

    pub fn new_variable_index_segment(variable_index: char) -> Self {
        PathSegment {
            id: ID::new(),
            kind: PathSegmentKind::VariableIndex(variable_index),
        }
    }

    pub fn to_string(&self) -> String {
        match &self.kind {
            PathSegmentKind::Key(key) => key.to_string(),
            PathSegmentKind::Index(index) => format!("[{}]", index),
            PathSegmentKind::VariableIndex(var) => format!("[{}]", var),
        }
    }
}
