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

