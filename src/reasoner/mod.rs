use async_trait::async_trait;

use crate::prelude::*;

pub struct CompletionMetadata {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[async_trait]
pub trait Reasoner: Send + Sync + Sized + 'static {
    async fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: Option<serde_json::Value>
    ) -> Result<(String, CompletionMetadata), Errors>;
}
