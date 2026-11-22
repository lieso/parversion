use std::fmt;

#[derive(Clone, Debug)]
pub enum ReasonerErrors {
    Unreasonable(String),
    InsufficientBackendQuota(String),
    RateLimitError(String),
    TransientBackendError(String),
    UnavailableSystemPrompt(String),
    EmbeddingError(String),
}

impl fmt::Display for ReasonerErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasonerErrors::Unreasonable(s) => write!(f, "Unreasonable: {}", s),
            ReasonerErrors::InsufficientBackendQuota(msg) => {
                write!(f, "insufficient backend quota: {}", msg)
            }
            ReasonerErrors::RateLimitError(msg) => write!(f, "rate limited: {}", msg),
            ReasonerErrors::TransientBackendError(msg) => {
                write!(f, "transient backend error: {}", msg)
            }
            ReasonerErrors::UnavailableSystemPrompt(msg) => {
                write!(f, "unavailable system prompt: {}", msg)
            }
            ReasonerErrors::EmbeddingError(msg) => write!(f, "embedding error: {}", msg),
        }
    }
}
