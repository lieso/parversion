use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};

use crate::prelude::*;
use crate::reasoner::{Reasoner, CompletionMetadata};

pub struct OpenRouterReasoner {
    client: OpenRouterClient,
}

impl OpenRouterReasoner {
    pub fn new() -> Self {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        let client = OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build OpenRouter client");
        OpenRouterReasoner { client }
    }
}

#[async_trait]
impl Reasoner for OpenRouterReasoner {
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
        }
        
        let response_format = ResponseFormat::json_schema(
            "whatdoIputhere?",
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

        match client.send_chat_completion(&request).await {
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

                    return Ok(content);
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

        todo!()
    }
}
