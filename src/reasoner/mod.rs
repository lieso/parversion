use async_trait::async_trait;

use crate::prelude::*;

#[cfg(feature = "openrouter-reasoner")]
pub mod openrouter;

pub struct CompletionMetadata {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub enum Capability {
    Fast,
    Capable,
}

#[async_trait]
pub trait Reasoner: Send + Sync + Sized + 'static {
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
}
