use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::path_segment::PathSegment;

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

    pub fn to_string(&self) -> String {
        self.segments.iter().fold(
            String::new(),
            |acc, segment| {
                format!("{}{}", acc, segment.to_string())
            })
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

        let mut path = Path::new();
        let path_segments = json_path.trim_start_matches('$').split('.');

        for segment in path_segments {
            let maybe_bracket = segment.find('[');

            if maybe_bracket.is_none() && !segment.is_empty() {
                path = path.with_key_segment(segment.to_string());
            }

            if let Some(bracket_start) = maybe_bracket {
                let key = &segment[..bracket_start];

                if !key.is_empty() {
                    path = path.with_key_segment(segment.to_string());
                }

                let bracket_end = segment.find(']').unwrap();

                let brackets_content = &segment[bracket_start..bracket_end];

                if brackets_content.is_empty() {
                    path = path.with_variable_index_segment();
                } else {
                    let index: usize = brackets_content.parse().unwrap();
                    path = path.with_index_segment(index);
                }
            }
        }

        path
    }
    
    pub fn arrayify(&mut self, target_segment_id: &ID) {
        if let Some(position) = self.segments.iter().position(|segment| segment.id == *target_segment_id) {
            self.segments.insert(position + 1, PathSegment::new_variable_index_segment('x'));
        } else {
            panic!("Could not find segment with id: {}", target_segment_id.to_string());
        }
    }
}
