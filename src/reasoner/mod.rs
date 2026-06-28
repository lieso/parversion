use async_trait::async_trait;
use std::sync::{Arc};

use crate::prelude::*;
use crate::classification::Classification;
use crate::prompt_registry::PromptRegistry;

mod backend;
mod classify;

#[cfg(feature = "openrouter-reasoner")]
pub use backend::openrouter;

pub struct CompletionMetadata {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

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
        capability: Capability,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value
    ) -> Result<(String, CompletionMetadata), Errors>;

    async fn execute<T: for<'de> serde::Deserialize<'de>>(
        &self,
        capability: Capability,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value
    ) -> Result<(T, CompletionMetadata), Errors> {
        let (content, metadata) = self.complete(
            capability,
            system_prompt,
            user_prompt,
            schema
        ).await?;

        let parsed = serde_json::from_str::<T>(&content).map_err(|e| {
            log::error!("Failed to parse reasoner response: {}", e);
            Errors::UnexpectedError
        })?;

        Ok((parsed, metadata))
    }

    async fn classify(
        &self,
        meta_context: Arc<MetaContext>,
    ) -> Result<(Classification, ReasonerMetadata), Errors> {
        Ok(classify::classify(self, meta_context).await?)
    }
}
