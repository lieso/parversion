use async_trait::async_trait;
use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};

use crate::prelude::*;
use crate::reasoner::{Reasoner, CompletionMetadata, Capability};
use crate::environment::get_env_variable;
use crate::prompt_registry::PromptRegistry;

#[cfg(feature = "openrouter-reasoner")]
pub struct OpenRouterReasoner {
    client: OpenRouterClient,
    prompts: PromptRegistry
}

#[cfg(feature = "openrouter-reasoner")]
impl OpenRouterReasoner {
    pub fn new(prompts: PromptRegistry) -> Self {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        let client = OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build OpenRouter client");
        OpenRouterReasoner { client, prompts }
    }
}

#[async_trait]
#[cfg(feature = "openrouter-reasoner")]
impl Reasoner for OpenRouterReasoner {
    fn prompts(&self) -> &PromptRegistry { &self.prompts }

    async fn complete(
        &self,
        capability: Capability,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
    ) -> Result<(String, CompletionMetadata), Errors> {
        log::debug!("");
        log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
        log::debug!("{}", system_prompt);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");
        log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
        log::debug!("{}", user_prompt);
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");

        let model = match capability {
            Capability::Fast => "gpt-5-mini",
            Capability::Capable => "gpt-5",
        };
        
        let response_format = ResponseFormat::json_schema(
            "structured_response",
            true,
            schema,
        );

        let request = ChatCompletionRequest::builder()
            .model(model)
            .messages(vec![
                Message::new(Role::System, system_prompt),
                Message::new(Role::User, user_prompt),
            ])
            .response_format(response_format)
            .build()
            .expect("Could not construct ChatCompletionRequest");

        match self.client.send_chat_completion(&request).await {
            Ok(response) => {
                log::debug!("┌─── RAW LLM RESPONSE ──────────────────────────────────────────┐");
                log::debug!("{:?}", response);
                log::debug!("└───────────────────────────────────────────────────────────────┘");
                log::debug!("");

                if let Some(content) = response.choices[0].content() {
                    log::debug!(
                        "┌─── LLM RESPONSE CONTENT ──────────────────────────────────────┐"
                    );
                    log::debug!("{}", content);
                    log::debug!(
                        "└───────────────────────────────────────────────────────────────┘"
                    );
                    log::debug!("");

                    let metadata = if let Some(usage) = response.usage {
                        CompletionMetadata {
                            input_tokens: usage.prompt_tokens as u32,
                            output_tokens: usage.completion_tokens as u32,
                        }
                    } else {
                        CompletionMetadata { input_tokens: 0, output_tokens: 0 }
                    };

                    return Ok((content.to_string(), metadata));
                } else {
                    log::error!(
                        "╔═══════════════════════════════════════════════════════════════╗"
                    );
                    log::error!(
                        "║                    NO CONTENT ERROR                           ║"
                    );
                    log::error!(
                        "╚═══════════════════════════════════════════════════════════════╝"
                    );
                    log::error!("No content in LLM response");
                    Err(Errors::UnexpectedError)
                }
            },
            Err(e) => {
                log::error!("╔═══════════════════════════════════════════════════════════════╗");
                log::error!("║                    REQUEST ERROR                              ║");
                log::error!("╚═══════════════════════════════════════════════════════════════╝");
                log::error!("Failed to get response from OpenRouter: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }
}
