use reqwest::header;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(feature = "caching")]
use crate::cache::Cache;
use crate::environment::get_env_variable;
use crate::prelude::*;

pub struct OpenAI;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ShouldEliminateCodeResponse {
    pub is_query_or_mutation: bool,
    pub justification: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CodeToHttpResponseHeaders {
    pub content_type: String,
    pub accept: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CodeToHttpResponse {
    method: String,
    url: String,
    headers: CodeToHttpResponseHeaders,
    query_params: Vec<String>,
    body: Vec<String>,
}

impl OpenAI {
    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        log::trace!("In function_to_operation");

        if code.len() > 20000 {
            log::debug!("{}", code);
            return Err(Errors::ContextTooLarge);
        }

        let eliminate_response = Self::should_eliminate_code(&code).await?;

        log::debug!("eliminate_response: {:?}", eliminate_response);

        if !eliminate_response.is_query_or_mutation {
            log::info!("Function is not a query or mutation");
            return Ok(None);
        }

        let operation_response = Self::code_to_http(&code).await?;

        log::debug!("operation_response: {:?}", operation_response);

        if operation_response.url.len() == 0 {
            log::warn!("Unexpected blank operation");

            // TODO
            /*
            function fn() {
                const v0 = document.createElement("link").relList;
                if (v0 && v0.supports && v0.supports("modulepreload")) return;
                for (const j of document.querySelectorAll('link[rel="modulepreload"]'))o(j);
                new MutationObserver((j)=>{
                    for (const w of j)if (w.type === "childList") for (const sl of w.addedNodes)sl.tagName === "LINK" && sl.rel === "modulepreload" && o(sl);
                }).observe(document, {
                    childList: !0,
                    subtree: !0
                });
                function $(p0) {
                    const v0 = {};
                    return p0.integrity && (v0.integrity = p0.integrity), p0.referrerPolicy && (v0.referrerPolicy = p0.referrerPolicy), p0.crossOrigin === "use-credentials" ? v0.credentials = "include" : p0.crossOrigin === "anonymous" ? v0.credentials = "omit" : v0.credentials = "same-origin", v0;
                }
                function o(p0) {
                    if (p0.ep) return;
                    p0.ep = !0;
                    const v0 = $(p0);
                    fetch(p0.href, v0);
                }
            }

            The function contains a call to fetch(p0.href, v0), which is a standard JavaScript method for making HTTP requests to a remote server. This is a direct query or mutation operation, as it sends a request to the URL specified by p0.href, potentially retrieving or modifying data on a remote server.
            */

            return Ok(None);
        }

        unimplemented!()
    }

    async fn should_eliminate_code(code: &str) -> Result<ShouldEliminateCodeResponse, Errors> {
        let system_prompt = format!(
            r##"
Your task is to determine whether a pseudo-javascript function directly performs any kind of query or mutation on a remote server. 

Look for the presence of URLs, http methods, JSON payloads and other things normally required for javascript to perform a query or mutation.

Please include a justification for your response
        "##
        );
        let user_prompt = format!("{}", code);

        let response_format = json!({
            "type": "json_schema",
            "name": "query_or_mutation",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "is_query_or_mutation": {
                        "type": "boolean"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["is_query_or_mutation", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(&system_prompt, &user_prompt, response_format).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE CODE START         ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE CODE END          ║");
                log::debug!("╚═══════════════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn code_to_http(code: &str) -> Result<CodeToHttpResponse, Errors> {
        let system_prompt = format!(
            r##"
Your task is to convert a pseudo-javascript function into a json object that contains all the information needed to reconstruct the network request or API call.

If you do not see an appropriate value for any of the keys in the response format, of if you do not think the code represents a network request, please leave blank.

Please include a justification for your response.
        "##
        );
        let user_prompt = format!("{}", code);

        let response_format = json!({
            "type": "json_schema",
            "name": "js_http",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string"
                    },
                    "url": {
                        "type": "string"
                    },
                    "headers": {
                        "type": "object",
                        "properties": {
                            "content_type": {
                                "type": "string"
                            },
                            "accept": {
                                "type": "string"
                            }
                        },
                        "required": ["content_type", "accept"],
                        "additionalProperties": false
                    },
                    "query_params": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "body": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": [
                    "method",
                    "url",
                    "headers",
                    "query_params",
                    "body",
                    "justification",
                ],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(&system_prompt, &user_prompt, response_format).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════════════════╗");
                log::debug!("║    CODE TO HTTP START                  ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║    CODE TO HTTP END                   ║");
                log::debug!("╚═══════════════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn send_openai_request<T>(
        system_prompt: &str,
        user_prompt: &str,
        response_format: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        log::trace!("In send_openai_request");

        let mut hash = Hash::from_items(vec![
            system_prompt,
            user_prompt,
            &response_format.to_string(),
        ]);
        let hash = hash.finalize();

        let response = Self::get_or_set_cache(hash.clone(), || async {
            let openai_api_key = get_env_variable("OPENAI_API_KEY");

            let request_json = json!({
                "model": "gpt-4.1-2025-04-14",
                "temperature": 0,
                "input": [
                    {
                        "role": "system",
                        "content": system_prompt
                    },
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "text": {
                    "format": response_format,
                }
            });

            let url = "https://api.openai.com/v1/responses";
            let authorization = format!("Bearer {}", openai_api_key);
            let client = reqwest::Client::new();

            match client
                .post(url)
                .json(&request_json)
                .header(header::AUTHORIZATION, authorization)
                .send()
                .await
            {
                Ok(res) => {
                    log::trace!("okay response from openai");
                    log::debug!("res: {:?}", res);

                    match res.json::<serde_json::Value>().await {
                        Ok(json_response) => {
                            log::trace!("okay json from openai");
                            log::debug!("json_response: {:?}", json_response);

                            json_response["output"][0]["content"][0]["text"]
                                .as_str()
                                .map(String::from)
                        }
                        Err(e) => {
                            log::error!("Failed to parse JSON response: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to send request to OpenAI: {}", e);
                    None
                }
            }
        })
        .await;

        let json_response = response.ok_or("Failed to get response from OpenAI")?;
        let parsed_response: T = serde_json::from_str(&json_response)?;
        Ok(parsed_response)
    }

    async fn get_or_set_cache<F, Fut>(_hash: Hash, fetch_data: F) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        #[cfg(feature = "caching")]
        {
            Cache::get_or_set_cache(hash, fetch_data).await
        }

        #[cfg(not(feature = "caching"))]
        {
            log::debug!("caching is disabled");
            fetch_data().await
        }
    }
}
