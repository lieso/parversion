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
    // LLM::get_node_groups returns lineage classifications keyed by lineage string; this error
    // fires when converting those strings back to Lineage objects via reverse lookup fails,
    // meaning the LLM returned a lineage string that was not present in the input.
    LineageConversionError(String),
}

impl From<JoinError> for Errors {
    fn from(_: JoinError) -> Self {
        Errors::UnexpectedError
    }
}
