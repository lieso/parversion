use crate::basis_graph::{BasisGraph};
use crate::document_profile::DocumentProfile;
use crate::transformation::{Transformation};

#[derive(Clone, Debug)]
pub enum AnalysisMode {
    DISABLED,
    SIMPLE,
    COMPLEX,
}

#[derive(Clone, Debug)]
pub enum Errors {
    FileReadError,
    FileInputError,
    FileOutputError,
    JsonParseError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnexpectedOutputFormat,
    XmlParseError,
    BasisGraphBuildError(String),
    PathConversionError,
    SqliteDatabaseConnectionError,
    YamlParseError,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub analysis_mode: Option<AnalysisMode>,
    //pub basis_graph: Option<BasisGraph>,
    //pub document_profile: Option<DocumentProfile>,
    pub origin: Option<String>,
    pub date: Option<String>,
    pub value_transformations: Option<Vec<Transformation>>
}

impl Default for Options {
    fn default() -> Self {
        Options {
            //basis_graph: None,
            //document_profile: None,
            analysis_mode: Some(AnalysisMode::COMPLEX),
            origin: None,
            date: None,
            value_transformations: None,
        }
    }
}
