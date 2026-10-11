use parversion::entrypoint::run;
use parversion::types::Errors;

fn build_runtime() -> tokio::runtime::Runtime {
    let mut builder = if std::env::var("SINGLE_THREAD").is_ok() {
        tokio::runtime::Builder::new_current_thread()
    } else {
        tokio::runtime::Builder::new_multi_thread()
    };
    builder.enable_all().build().unwrap()
}

fn main() {
    let runtime = build_runtime();
    if let Err(e) = runtime.block_on(run()) {
        handle_error(&e);
        std::process::exit(1);
    }
    std::process::exit(0);
}

fn handle_error(err: &Errors) {
    let message = match err {
        Errors::DocumentNotProvided => "no document provided".to_string(),
        Errors::UnexpectedDocumentType => "unexpected document type".to_string(),
        Errors::DocumentTypeNotProvided => "document type not provided".to_string(),
        Errors::FileInputError => "failed to read input file".to_string(),
        Errors::XmlParseError => "failed to parse XML".to_string(),
        Errors::YamlParseError(msg) => format!("failed to parse YAML: {}", msg),
        Errors::YamlProviderError => "YAML provider error".to_string(),
        Errors::JsonParseError(msg) => format!("failed to parse JSON: {}", msg),
        Errors::UnexpectedError(msg) => format!("unexpected error: {}", msg),
        Errors::PathConversionError => "failed to convert path".to_string(),
        Errors::FetchUrlError(msg) => format!("failed to fetch URL: {}", msg),
        Errors::DeficientNormalizationContextError(msg) => {
            format!("deficient normalization context: {}", msg)
        }
        Errors::ClassificationNotFound => "classification not found".to_string(),
        Errors::InsufficientPrerequisites(msg) => format!("insufficient prerequisites: {}", msg),
        Errors::XPathParseError(msg) => format!("failed to parse XPath: {}", msg),
        Errors::XPathTraverseError(msg) => format!("failed to traverse XPath: {}", msg),
        Errors::ProviderError(msg) => format!("provider error: {}", msg),
        Errors::UnexpectedParameter(msg) => format!("unexpected parameter: {}", msg),
        Errors::TooManyTranslationDocuments => "too many translation documents".to_string(),
        Errors::InvalidRole(msg) => format!("invalid role: {}", msg),
        Errors::ReasonerNotConfigured => "reasoner not configured".to_string(),
        Errors::PromptRegistryError(msg) => format!("prompt registry error: {}", msg),
        Errors::UnavailableSystemPrompt(msg) => format!("unavailable system prompt: {}", msg),
        Errors::InsufficientBackendQuota(msg) => format!("insufficient backend quota: {}", msg),
        Errors::RateLimitError(msg) => format!("rate limited: {}", msg),
        Errors::TransientBackendError(msg) => format!("transient backend error: {}", msg),
        Errors::EmbeddingError(msg) => format!("embedding error: {}", msg),
        Errors::TaskJoinError(msg) => format!("task join error: {}", msg),
        Errors::XsltParseError(msg) => format!("failed to parse XSLT: {}", msg),
    };

    eprintln!("Error: {}", message);
}
