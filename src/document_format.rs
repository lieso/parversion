use crate::document::DocumentType;

#[derive(Debug, Clone)]
enum FormatStrategy {
    Simple,
    Flattened,
    Nested,
}

#[derive(Debug, Clone)]
pub struct DocumentFormat {
    format_type: DocumentType,
    encoding: Option<String>,
    strategy: Option<FormatStrategy>,
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
            format_type: DocumentType::JSON,
            encoding: Some(String::from("UTF-8")),
            strategy: None,
            indent: None,
            line_ending: None,
            headers: None,
            wrap_text: None,
            exclude_nulls: None,
            custom_delimiter: None,
        }
    }
}
