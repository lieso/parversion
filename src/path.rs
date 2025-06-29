use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Path {
    pub segments: Vec<String>,
}

impl Path {
    pub fn new() -> Self {
        Path {
            segments: Vec::new()
        }
    }

    pub fn from_str(val: &str) -> Self {
        Path {
            segments: vec![val.to_string()]
        }
    }

    pub fn from_json_path(json_path: &String) -> Self {
        unimplemented!()
    }

    pub fn with_segment(&self, segment: String) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(segment);
        new_path
    }
}
