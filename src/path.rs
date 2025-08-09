use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PathSegment {
    Key(String),
    Index(usize),
    VariableIndex(char),
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
            segments: vec![PathSegment::Key(val.to_string())],
        }
    }

    pub fn with_key_segment(&self, key: String) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::Key(key));
        new_path
    }

    pub fn with_index_segment(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::Index(index));
        new_path
    }

    pub fn with_index_segment_increment(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::Index(index + 1));
        new_path
    }

    pub fn with_any_index_segment(&self) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::AnyIndex);
        new_path
    }

    pub fn from_json_path(json_path: &String) -> Self {
        Self::new()
    }
}
