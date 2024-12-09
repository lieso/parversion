
pub struct SchemaPath {
    pub segments: Vec<String>,
}

impl SchemaPath {
    pub fn to_string(self) -> {
        self.segments.join('.')
    }
}
