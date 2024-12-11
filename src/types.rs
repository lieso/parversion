use serde::{Serialize, Deserialize};

pub enum AnalysisMode {
    DISABLED,
    SIMPLE,
    COMPLEX,
}

pub struct Options {
    basis_graph: Option<BasisGraph>,
    analysis_mode: Option<AnalysisMode>,
    origin: Option<String>,
    date: Option<String>,
    value_transformations: Option<Vec<Transformation>>
}

pub type OutputData = Vec<JsonNode>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnexpectedOutputFormat,
    XmlParseError
}
