use crate::document::DocumentType;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocumentFormat {
    pub format_type: DocumentType,
    pub encoding: Option<String>,
    pub indent: Option<usize>,
    pub line_ending: Option<String>,
    pub headers: Option<bool>,
    pub wrap_text: Option<bool>,
    pub exclude_nulls: Option<bool>,
    pub custom_delimiter: Option<char>,
}

impl Default for DocumentFormat {
    fn default() -> Self {
        DocumentFormat {
            format_type: DocumentType::Json,
            encoding: Some(String::from("UTF-8")),
            indent: None,
            line_ending: None,
            headers: None,
            wrap_text: None,
            exclude_nulls: None,
            custom_delimiter: None,
        }
    }
}
