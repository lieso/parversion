use tokio::task::JoinError;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DocumentVersion {
    InputDocument,
    OrganizedDocument,
    NormalizedDocument,
}

#[derive(Clone, Debug)]
pub enum Errors {
    FileReadError,
    FileInputError,
    FileOutputError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    DocumentTypeNotProvided,
    UnexpectedError,
    XmlParseError,
    PathConversionError,
    YamlParseError(String),
    YamlProviderError,
    ProviderError(String),
    FetchUrlError(String),
    FieldTransformationFieldNotFound,
    GraphRootNotProvided,
    ProfileNotProvided,
    ContextsNotProvided,
    ClassificationNotProvided,
    ContextTooLarge,
    SchemaNotProvided,
    SchemaNotValid,
    JsonSchemaParseError(String),
    DeficientMetaContextError(String),
    DocumentVersionNotFound,
    ClassificationNotFound,
    OriginNotProvidedError,
    InsufficientPrerequisites(String),
    XPathParseError(String),
    XPathTraverseError(String),
}

impl From<JoinError> for Errors {
    fn from(_: JoinError) -> Self {
        Errors::UnexpectedError
    }
}
