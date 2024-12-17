use serde::{Serialize, Deserialize};

pub enum AnalysisMode {
    DISABLED,
    SIMPLE,
    COMPLEX,
}

pub enum DocumentType {
    Json,
    PlainText,
    Xml,
    Html,
}

pub struct Options {
    pub target_document_Type: Option<DocumentType>,
    pub basis_graph: Option<BasisGraph>,
    pub analysis_mode: Option<AnalysisMode>,
    pub origin: Option<String>,
    pub date: Option<String>,
    pub value_transformations: Option<Vec<Transformation>>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    FileInputError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnexpectedOutputFormat,
    XmlParseError
}
