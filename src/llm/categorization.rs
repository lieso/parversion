use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prelude::*;
use crate::environment::get_env_variable;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CategorizationResponse {
    pub description: String,
    pub structure: String,
    pub category: String,
    pub one_word_aliases: Vec<String>,
    pub two_word_aliases: Vec<String>,
}

pub struct Categorization;

impl Categorization {
    pub async fn categorize_graph(document: &str) -> Result<CategorizationResponse, Errors> {
        log::trace!("In categorize_graph");

        let system_prompt = r##"
Your task is to analyze a condensed web page, extrapolate from this minimized version, and provide the following information about the original website the condensed document was derived from:

1. (description): A short paragraph describing this web page.
2. (structure): A detailed description on how the HTML of the page is structured and the way content is organized from a technical perspective.
3. (category): Use one to two words in snake case to categorize this type of website. Emphasize in your categorization the type of user interface it is, and not so much the categorization of its content.
4. (one_word_aliases): Provide an additional ten categories, using one word, that best fit this type of website or user interface.
4. (two_word_aliases): Provide an additional twenty categories, using two words in snake case, that best fit this type of website or user interface.
              "##;

              let user_prompt = format!(r##"
[Document]
{}
"##, document);

              log::debug!("╔═══════════════════════════════════════════════════════════════╗");
              log::debug!("║                                                               ║");
              log::debug!("║              CATEGORIZE GRAPH - LLM REQUEST                   ║");
              log::debug!("║                                                               ║");
              log::debug!("╚═══════════════════════════════════════════════════════════════╝");
              log::debug!("");
              log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
              log::debug!("{}", system_prompt);
              log::debug!("└───────────────────────────────────────────────────────────────┘");
              log::debug!("");
              log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
              log::debug!("{}", user_prompt);
              log::debug!("└───────────────────────────────────────────────────────────────┘");
              log::debug!("");

              let client = Self::build_client();

              let response_format = ResponseFormat::json_schema(
                  "summarize_categorize_document",
                  true,
                  json!({
                      "type": "object",
                      "properties": {
                          "description": {
                              "type": "string",
                              "description": "Description of web page"
                          },
                          "structure": {
                              "type": "string",
                              "description": "Technical description of web page"
                          },
                          "category": {
                              "type": "string",
                              "description": "Categorization of web page"
                          },
                          "one_word_aliases": {
                              "type": "array",
                              "description": "Array of category aliases",
                              "items": {
                                  "type": "string",
                                  "description": "An alias of the main category"
                              }
                          },
                          "two_word_aliases": {
                              "type": "array",
                              "description": "Array of category aliases",
                              "items": {
                                  "type": "string",
                                  "description": "An alias of the main category"
                              }
                          }
                      },
                      "required": ["description", "structure", "category", "one_word_aliases", "two_word_aliases"],
                      "additionalProperties": false
                  }),
              );

            let request = ChatCompletionRequest::builder()
                .model("google/gemini-3-pro-preview")
                .messages(vec![
                    Message::new(Role::System, system_prompt),
                    Message::new(Role::User, user_prompt),
                ])
                .response_format(response_format)
                .build()
                .expect("Could not create llm request");

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

                        match serde_json::from_str::<CategorizationResponse>(content) {
                            Ok(parsed_response) => {
                                log::debug!(
                                    "┌─── PARSED RESPONSE ───────────────────────────────────────────┐"
                                );
                                log::debug!("{:?}", parsed_response);
                                log::debug!(
                                    "└───────────────────────────────────────────────────────────────┘"
                                );
                                log::debug!("");
                                log::debug!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "║            CATEGORIZE DOCUMENT - REQUEST COMPLETE             ║"
                                );
                                log::debug!(
                                    "║                                                               ║"
                                );
                                log::debug!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );

                                Ok(parsed_response)
                            }
                            Err(e) => {
                                log::error!(
                                    "╔═══════════════════════════════════════════════════════════════╗"
                                );
                                log::error!(
                                    "║                    PARSE ERROR                                ║"
                                );
                                log::error!(
                                    "╚═══════════════════════════════════════════════════════════════╝"
                                );
                                log::error!("Failed to parse LLM response: {}", e);
                                Err(Errors::UnexpectedError)

                            }
                        }
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
                }
                Err(e) => {
                    log::error!("╔═══════════════════════════════════════════════════════════════╗");
                    log::error!("║                    REQUEST ERROR                              ║");
                    log::error!("╚═══════════════════════════════════════════════════════════════╝");
                    log::error!("Failed to get response from OpenRouter: {}", e);
                    Err(Errors::UnexpectedError)
                }
            }
    }

    fn build_client() -> OpenRouterClient {
        let api_key = get_env_variable("OPENROUTER_API_KEY");
        OpenRouterClient::builder()
            .api_key(api_key)
            .build()
            .expect("Could not build open router client")
    }
}
