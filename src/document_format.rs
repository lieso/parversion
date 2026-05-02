use crate::document::DocumentType;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocumentFormat {
    pub format_type: DocumentType,
    encoding: Option<String>,
    indent: Option<usize>,
    line_ending: Option<String>,
    headers: Option<bool>,
    wrap_text: Option<bool>,
    exclude_nulls: Option<bool>,
    custom_delimiter: Option<char>,
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
