use serde::{Serialize, Deserialize};
use serde_json::{json, Value, Map};
use std::collections::{HashMap};

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

    pub fn to_string(&self) -> String {
        match (self.key.clone(), self.index, self.variable_index) {
            (Some(key), None, None) => {
                key.to_string()
            }
            (None, Some(index), None) => {
                format!("[{}]", index)
            }
            (None, None, Some(_variable_index)) => {
                "[]".to_string()
            }
            _ => {
                panic!("Invalid PathSegment struct received");
            }
        }
    }

    pub fn is_array_segment(&self) -> bool {
        self.index.is_some() || self.variable_index.is_some()
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
        log::debug!("json_path: {}", json_path);

        let mut path = Path::new();
        let path_segments = json_path.trim_start_matches('$').split('.');

        for segment in path_segments {
            let maybe_bracket = segment.find('[');

            if maybe_bracket.is_none() && !segment.is_empty() {
                log::debug!("Found key: {}", segment.to_string());
                path = path.with_key_segment(segment.to_string());
            }

            if let Some(bracket_start) = maybe_bracket {
                let key = &segment[..bracket_start];

                if !key.is_empty() {
                    log::debug!("Found key: {}", key.to_string());
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
    
    pub fn merge_path(&self, path: Path) -> Self {
        log::trace!("In merge_path");

        let mut variable_index_counter = 0;

        let merged_segments: Vec<PathSegment> = self.segments.iter().map(|segment| {
            if segment.variable_index.is_some() {
                let index_segments: Vec<&PathSegment> = path.segments
                    .iter()
                    .filter(|item| item.index.is_some())
                    .collect();

                if let Some(index) = index_segments.get(variable_index_counter) {
                    variable_index_counter += 1;

                    return (*index).clone();
                }
            }

            segment.clone()
        }).collect();

        Path { segments: merged_segments }
    }

    pub fn arrayify(&mut self, target_segment_id: &ID) {
        if let Some(position) = self.segments.iter().position(|segment| segment.id == *target_segment_id) {
            self.segments.insert(position + 1, PathSegment::new_variable_index_segment('x'));
        } else {
            panic!("Could not find segment with id: {}", target_segment_id.to_string());
        }
    }

    pub fn insert_into_map(
        &self,
        map: &mut Map<String, Value>,
        insert_key: String,
        insert_value: String,
    ) {
        fn recurse<'a>(
            map: &'a mut Map<String, Value>,
            segments: &'a [PathSegment],
        ) -> &'a mut Map<String, Value> {
            if let Some(first_segment) = segments.first() {
                let key = first_segment.key.as_ref().unwrap();

                if let Some(second_segment) = segments.get(1) {
                    if second_segment.is_array_segment() {
                        let next_value = map
                            .entry(key.clone())
                            .or_insert_with(|| Value::Array(Vec::new()));

                        if let Value::Array(ref mut array) = next_value {
                            if array.is_empty() || !matches!(array.last(), Some(Value::Object(_))) {
                                array.push(Value::Object(Map::new()));
                            }

                            let last_object = array
                                .last_mut()
                                .and_then(|v| if let Value::Object(ref mut obj) = *v {
                                    Some(obj)
                                } else {
                                    None
                                })
                                .unwrap();

                            let remaining_segments = &segments[2..];
                            return recurse(last_object, remaining_segments);
                        } else {
                            panic!("Expected an array");
                        }
                    }
                }

                let next_value = map
                    .entry(key.clone())
                    .or_insert_with(|| Value::Object(Map::new()));

                let next_map = if let Value::Object(ref mut obj) = next_value {
                    obj
                } else {
                    panic!("Expected an object");
                };

                let remaining_segments = &segments[1..];

                recurse(
                    next_map,
                    remaining_segments
                )
            } else {
                map
            }
        }

        let object_at_path = recurse(
            map,
            &self.segments,
        );

        object_at_path
            .entry(insert_key.clone())
            .or_insert_with(|| Value::String(insert_value.clone()));

        log::debug!("map: {:?}", map);
    }
}
