use tokio::task::JoinError;

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
    YamlParseError,
    FetchUrlError(String),
    BasisNodeNotFound,
    FieldTransformationFieldNotFound
}

impl From<JoinError> for Errors {
    fn from(_: JoinError) -> Self {
        Errors::UnexpectedError
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub origin: Option<String>,
    pub date: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            origin: None,
            date: None,
        }
    }
}
