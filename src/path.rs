use serde::{Serialize, Deserialize};
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::path_segment::{PathSegment, PathSegmentKind};

pub const AVAILABLE_VARIABLES: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'
];

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

        let mut used_variables = Vec::new();

        let re = Regex::new(r"[^.\[]+|\[[^\]]*\]").unwrap();

        for cap in re.find_iter(path) {
            let segment = cap.as_str();

            if segment.starts_with('[') {
                let content = &segment[1..segment.len() - 1];

                if content.is_empty() {
                    let variable = AVAILABLE_VARIABLES.iter()
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
        self.segments.iter().enumerate().fold(
            String::new(),
            |acc, (i, segment)| {
                let needs_dot = i > 0 && matches!(segment.kind, PathSegmentKind::Key(_));
                let prefix = if needs_dot { "." } else { "" };
                format!("{}{}{}", acc, prefix, segment.to_string())
            })
    }

    pub fn map_variables_to_indices(variable_path: &Path, index_path: &Path) -> Result<HashMap<char, usize>, Errors> {
        if variable_path.segments.len() != index_path.segments.len() {
            log::error!("Paths have different lengths");
            return Err(Errors::UnexpectedError);
        }

        let mut mapping = HashMap::new();

        for (var_segment, idx_segment) in variable_path.segments.iter().zip(index_path.segments.iter()) {
            match (&var_segment.kind, &idx_segment.kind) {
                (PathSegmentKind::Key(key1), PathSegmentKind::Key(key2)) => {
                    if key1 != key2 {
                        log::error!("Key mismatch: {} != {}", key1, key2);
                        return Err(Errors::UnexpectedError);
                    }
                }
                (PathSegmentKind::VariableIndex(var), PathSegmentKind::Index(idx)) => {
                    mapping.insert(*var, *idx);
                }
                (PathSegmentKind::Index(idx1), PathSegmentKind::Index(idx2)) => {
                    if idx1 != idx2 {
                        log::error!("Index mismatch: {} != {}", idx1, idx2);
                        return Err(Errors::UnexpectedError);
                    }
                }
                _ => {
                    log::error!("Segment type mismatch at position");
                    return Err(Errors::UnexpectedError);
                }
            }
        }

        Ok(mapping)
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

    pub fn with_segment(&self, segment: PathSegment) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(segment);
        new_path
    }

    pub fn with_unique_variables(&self, other: &Path) -> Self {
        // Collect all variable characters used in the other path
        let mut used_variables: HashSet<char> = other.segments
            .iter()
            .filter_map(|seg| {
                if let PathSegmentKind::VariableIndex(var) = seg.kind {
                    Some(var)
                } else {
                    None
                }
            })
            .collect();

        // Mapping from old variables to new variables
        let mut variable_mapping: HashMap<char, char> = HashMap::new();

        // Create new path with unique variables
        let mut new_path = Path::new();

        for segment in &self.segments {
            match &segment.kind {
                PathSegmentKind::Key(key) => {
                    new_path = new_path.with_key_segment(key.clone());
                }
                PathSegmentKind::Index(idx) => {
                    new_path = new_path.with_index_segment(*idx);
                }
                PathSegmentKind::VariableIndex(var) => {
                    // Check if we already mapped this variable
                    let new_var = if let Some(&mapped_var) = variable_mapping.get(var) {
                        mapped_var
                    } else {
                        // Find a new unique variable
                        let new_var = AVAILABLE_VARIABLES
                            .iter()
                            .find(|&v| !used_variables.contains(v))
                            .copied()
                            .expect("Ran out of available variable characters");

                        used_variables.insert(new_var);
                        variable_mapping.insert(*var, new_var);
                        new_var
                    };

                    new_path = new_path.with_variable_index_segment(new_var);
                }
            }
        }

        new_path
    }

    pub fn with_mapped_variables(&self, mapping: &HashMap<char, PathSegmentKind>) -> Self {
        let mut new_path = Path::new();

        for segment in &self.segments {
            match &segment.kind {
                PathSegmentKind::Key(key) => {
                    new_path = new_path.with_key_segment(key.clone());
                }
                PathSegmentKind::Index(idx) => {
                    new_path = new_path.with_index_segment(*idx);
                }
                PathSegmentKind::VariableIndex(var) => {
                    let new_segment = if let Some(mapped_segment_kind) = mapping.get(var) {
                        PathSegment {
                            id: ID::new(),
                            kind: mapped_segment_kind.clone()
                        }
                    } else {
                        segment.clone()
                    };

                    new_path = new_path.with_segment(new_segment);
                }
            }
        }

        new_path
    }

    pub fn arrayify(&mut self, target_segment_id: &ID, variable: char) {
        unimplemented!()
    }
}
