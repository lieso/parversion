use serde::{Serialize, Deserialize};
use regex::Regex;

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

    pub fn from_key(key: &str) -> Self {
        Path {
            segments: vec![PathSegment::new_key_segment(key.to_string())],
        }
    }

    // Parses bespoke json paths like: a.b[2].c.d[x].e[y].z[0]
    pub fn from_str(path: &str) -> Self {
        let path = path.trim_start_matches('$');
        let mut result = Path::new();

        let available_variables: Vec<char> = ('a'..='z').collect();
        let mut used_variables = Vec::new();

        let re = Regex::new(r"[^.\[]+|\[[^\]]*\]").unwrap();

        for cap in re.find_iter(path) {
            let segment = cap.as_str();

            if segment.starts_with('[') {
                let content = &segment[1..segment.len() - 1];

                if content.is_empty() {
                    let variable = available_variables.iter()
                        .find(|&v| !used_variables.contains(v))
                        .expect("Ran out of variable index characters");

                    used_variables.push(*variable);
                    result = result.with_variable_index_segment(*variable);
                } else if let Ok(index) = content.parse::<usize>() {
                    result = result.with_index_segment(index);
                } else {
                    let variable = content.chars().next().unwrap_or('a');
                    used_variables.push(variable);
                    result = result.with_variable_index_segment(variable);
                }
            } else {
                result = result.with_key_segment(segment.to_string());
            }
        }

        result
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

    pub fn with_variable_index_segment(&self, variable: char) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::new_variable_index_segment(variable));
        new_path
    }

    pub fn arrayify(&mut self, target_segment_id: &ID, variable: char) {
        unimplemented!()
    }
}
