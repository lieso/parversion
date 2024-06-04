use reqwest::header;
use serde_json::json;
use std::env;

pub async fn get_color_palette(screenshot_base64: String) {
    log::trace!("In get_color_palette");

    let prompt = format!(r##"
Look at this screenshot of a website and provide color palette that roughly matches the appearance of the website. Provide only three color hex codes for a primary, secondary and background color as JSON, and do not include any commentary, introduction or summary.

Here is an example of expected JSON response:

{{
    "primary_hex": "#7F92D9",
    "secondary_hex": "#E87AC2",
    "background_hex": "#3BCC85"
}}
"##);
    log::trace!("prompt: {}", prompt);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OpenAI API key has not been set!");
    let request_json = json!({
        "model":  "gpt-4o",
        "temperature":  0,
        "messages":  [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                          "url": "https://davidroussov.s3.amazonaws.com/hn.png"
                        }
                    }
                ]
            }
        ]
    });
    let url = "https://api.openai.com/v1/chat/completions";
    let authorization = format!("Bearer {}", openai_api_key);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&request_json)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, authorization)
        .send()
        .await;
    log::trace!("response: {:?}", response);
    let response = response.expect("Failed to send request to OpenAI");

    let json_response = response.json::<serde_json::Value>().await.expect("Unable to get JSON from response");
    log::debug!("json_response: {:?}", json_response);
    let json_response = json_response["choices"].as_array().unwrap();
    let json_response = &json_response[0]["message"]["content"].as_str().unwrap();
    let json_response = fix_json_response(json_response);
    log::debug!("json_response: {:?}", json_response);
}

fn fix_json_response(response: &str) -> &str {
    let without_backticks = remove_codeblock_delimiters(response);
    let without_label = remove_label_text(without_backticks);

    return without_label;
}

fn remove_codeblock_delimiters(s: &str) -> &str {
     if s.starts_with("```") && s.ends_with("```") {
         s.trim_start_matches("```").trim_end_matches("```")
     } else {
         s
     }
}

fn remove_label_text(s: &str) -> &str {
    if s.starts_with("json") {
        return &s[4..];
    }

    s
}
