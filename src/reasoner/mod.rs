use async_trait::async_trait;

use crate::prelude::*;

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
}
