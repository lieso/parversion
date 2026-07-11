use tokio::task::JoinError;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DocumentVersion {
    InputDocument,
    OrganizedDocument,
}

#[derive(Clone, Debug)]
pub enum Errors {
    FileInputError,
    FileOutputError,
    YamlParseError(String),
    JsonParseError(String),
    DocumentNotProvided,
    UnexpectedDocumentType,
    DocumentTypeNotProvided,
    UnexpectedError,
    XmlParseError,
    PathConversionError,
    FetchUrlError(String),
    FieldTransformationFieldNotFound,
    ContextTooLarge,
    DeficientMetaContextError(String),
    DeficientNormalizationContextError(String),
    DeficientTranslationContextError(String),
    DocumentVersionNotFound,
    ClassificationNotFound,
    OriginNotProvidedError,
    InsufficientPrerequisites(String),
    XPathParseError(String),
    XPathTraverseError(String),
    YamlProviderError,
    ProviderError(String),
    UnexpectedParameter(String),
    TooManyTranslationDocuments,
    InvalidRole(String),
    ReasonerNotConfigured,
    PromptRegistryError(String),
    UnavailableSystemPrompt(String),
    InsufficientBackendQuota(String),
    RateLimitError(String),
    TransientBackendError(String),
    RequestTimeout(String),
    EmbeddingError(String),
}

impl From<JoinError> for Errors {
    fn from(_: JoinError) -> Self {
        Errors::UnexpectedError
    }
}
