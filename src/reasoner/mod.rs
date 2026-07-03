use async_trait::async_trait;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::classification::Classification;
use crate::prompt_registry::PromptRegistry;
use crate::basis_field::BasisField;
use crate::basis_group::BasisGroup;

mod backend;
mod classify;
mod basis_field;
mod basis_group;

#[cfg(feature = "openrouter-reasoner")]
pub use backend::openrouter;

pub struct CompletionMetadata {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub struct EmbeddingMetadata {
    pub input_tokens: u32,
}

#[derive(Debug)]
pub enum Capability {
    Fast,
    Capable,
}

pub struct ReasonerMetadata {
    pub tokens: u32,
}

#[async_trait]
pub trait Reasoner: Send + Sync + Sized + 'static {
    fn prompts(&self) -> &PromptRegistry;

    async fn complete(
        &self,
        capability: &Capability,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value
    ) -> Result<(String, CompletionMetadata), Errors>;

    async fn execute<T: for<'de> serde::Deserialize<'de>>(
        &self,
        capability: &Capability,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value
    ) -> Result<(T, CompletionMetadata), Errors> {
        let mut backoff = std::time::Duration::from_millis(100);
        let max_backoff = std::time::Duration::from_secs(30);
        let max_retries = 5;

        for attempt in 0..=max_retries {
            match self.complete(
                capability,
                system_prompt,
                user_prompt,
                schema.clone()
            ).await {
                Ok((content, metadata)) => {
                    let parsed = serde_json::from_str::<T>(&content).map_err(|e| {
                        log::error!("Failed to parse reasoner response: {}", e);
                        Errors::UnexpectedError
                    })?;

                    return Ok((parsed, metadata));
                }
                Err(e) if is_retryable(&e) => {
                    log::warn!("Retryable error on attempt {}, backing off: {:?}", attempt + 1, e);
                    let jitter = std::time::Duration::from_millis(rand::random::<u64>() % 100);
                    tokio::time::sleep(backoff + jitter).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
                Err(e) => return Err(e),
            }
        }
        
        unreachable!()
    }

    async fn embed(
        &self,
        inputs: Vec<String>
    ) -> Result<(Vec<Vec<f32>>, EmbeddingMetadata), Errors>;

    async fn classify(
        &self,
        meta_context: Arc<MetaContext>,
    ) -> Result<(Classification, ReasonerMetadata), Errors> {
        Ok(classify::classify(self, meta_context).await?)
    }

    async fn basis_field(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        group: Vec<Arc<Context>>,
        candidate: String
    ) -> Result<(Option<BasisField>, ReasonerMetadata), Errors> {
        Ok(basis_field::basis_field(self, normalization_context, group, candidate).await?)
    }

    async fn basis_group(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        group: Vec<Arc<Context>>,
    ) -> Result<(Option<BasisGroup>, ReasonerMetadata), Errors> {
        Ok(basis_group::basis_group(self, normalization_context, group).await?)
    }
}

fn is_retryable(error: &Errors) -> bool {
    matches!(error,
        Errors::RateLimitError(_)
        | Errors::TransientBackendError(_)
        | Errors::RequestTimeout(_)
    )
}
