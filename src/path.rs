use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathSegment {
    pub id: ID,
    pub key: Option<String>,
    pub index: Option<usize>,
    pub variable_index: Option<char>,
}

impl PathSegment {
    fn new_key_segment(key: String) -> Self {
        PathSegment {
            id: ID::new(),
            key: Some(key),
            index: None,
            variable_index: None,
        }
    }

    fn new_index_segment(index: usize) -> Self {
        PathSegment {
            id: ID::new(),
            key: None,
            index: Some(index),
            variable_index: None,
        }
    }

    fn new_variable_index_segment(variable_index: char) -> Self {
        PathSegment {
            id: ID::new(),
            key: None,
            index: None,
            variable_index: Some(variable_index),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

impl Path {
    pub fn new() -> Self {
        Path {
            segments: Vec::new()
        }
    }

    pub fn from_str(val: &str) -> Self {
        Path {
            segments: vec![PathSegment::new_key_segment(val.to_string())],
        }
    }

    pub fn with_key_segment(&self, key: String) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::new_key_segment(key));
        new_path
    }

    pub fn with_index_segment(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::new_index_segment(index));
        new_path
    }

    pub fn with_index_segment_increment(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::new_index_segment(index + 1));
        new_path
    }

    pub fn with_variable_index_segment(&self) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::new_variable_index_segment('x'));
        new_path
    }

    pub fn from_json_path(json_path: &String) -> Self {
        Self::new()
    }

    pub fn arrayify(&mut self, target_segment_id: &ID) {
        if let Some(position) = self.segments.iter().position(|segment| segment.id == *target_segment_id) {
            self.segments.insert(position + 1, PathSegment::new_variable_index_segment('x'));
        } else {
            panic!("Could not find segment with id: {}", target_segment_id.to_string());
        }
    }
}
