use tokio::task::JoinError;

#[derive(Clone, Debug)]
pub enum Errors {
    FileReadError,
    FileInputError,
    FileOutputError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    XmlParseError,
    PathConversionError,
    YamlParseError(String),
    FetchUrlError(String),
    FieldTransformationFieldNotFound,
    GraphRootNotProvided,
    ProfileNotProvided,
    ContextsNotProvided,
    BasisGraphNotProvided,
    ContextTooLarge,
    SchemaNotProvided,
    JsonSchemaParseError(String),
    DeficientMetaContextError(String)
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
