use crate::prelude::*;
use crate::environment::get_env_variable;
use openrouter_rs::{OpenRouterClient, api::chat::*, types::Role};

pub struct Translation;

impl Translation {
    pub async fn match_target_schema(
        marked_schema_node: &String,
        target_schema: &String
    ) -> Result<Option<String>, Errors> {
        log::trace!("In match_target_schema");

        let client = Self::build_client();

        // Send chat completion
        let request = ChatCompletionRequest::builder()
            .model("anthropic/claude-sonnet-4")
            .messages(vec![
                Message::new(Role::User, "Explain Rust ownership in simple terms")
            ])
            .build()
            .expect("could not create request");

        let response = client.send_chat_completion(&request).await.expect("Could not send chat completion");
        println!("{}", response.choices[0].content().unwrap_or(""));


        unimplemented!()
    }

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
