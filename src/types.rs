use crate::basis_graph::{BasisGraph};
use crate::transformation::{Transformation};

#[derive(Clone, Debug)]
pub enum AnalysisMode {
    DISABLED,
    SIMPLE,
    COMPLEX,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub basis_graph: Option<BasisGraph>,
    pub analysis_mode: Option<AnalysisMode>,
    pub origin: Option<String>,
    pub date: Option<String>,
    pub value_transformations: Option<Vec<Transformation>>
}

#[derive(Clone, Debug)]
pub enum Errors {
    FileInputError,
    FileOutputError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnexpectedOutputFormat,
    XmlParseError,
    BasisGraphBuildError(String),
    PathConversionError,
}
