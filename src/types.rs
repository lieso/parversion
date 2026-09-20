use tokio::task::JoinError;

#[derive(Clone, Debug)]
pub enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    DocumentTypeNotProvided,
    FileInputError,
    XmlParseError,
    YamlParseError(String),
    YamlProviderError,
    JsonParseError(String),
    UnexpectedError(String),
    PathConversionError,
    FetchUrlError(String),
    DeficientMetaContextError(String),
    DeficientNormalizationContextError(String),
    DeficientTranslationContextError(String),
    ClassificationNotFound,
    InsufficientPrerequisites(String),
    XPathParseError(String),
    XPathTraverseError(String),
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
    TaskJoinError(String),
}

impl From<JoinError> for Errors {
    fn from(e: JoinError) -> Self {
        Errors::TaskJoinError(e.to_string())
    }
}
