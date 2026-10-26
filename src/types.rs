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
    DeficientNormalizationContextError(String),
    ClassificationNotFound,
    InsufficientPrerequisites(String),
    XPathParseError(String),
    XPathTraverseError(String),
    ProviderError(String),
    UnexpectedParameter(String),
    TooManyTranslationDocuments,
    InvalidRole(String),
    #[allow(dead_code)]
    ReasonerNotConfigured,
    PromptRegistryError(String),
    UnavailableSystemPrompt(String),
    InsufficientBackendQuota(String),
    RateLimitError(String),
    TransientBackendError(String),
    EmbeddingError(String),
    TaskJoinError(String),
    XsltParseError(String),
}

impl From<JoinError> for Errors {
    fn from(e: JoinError) -> Self {
        Errors::TaskJoinError(e.to_string())
    }
}

impl From<serde_json::Error> for Errors {
    fn from(e: serde_json::Error) -> Self {
        Errors::JsonParseError(e.to_string())
    }
}
