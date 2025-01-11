use std::io::{self, Read, Write};
use std::fs::File;
use std::path::{Path};
use fantoccini::{error::CmdError, ClientBuilder, Locator};

use crate::types::*;

pub fn get_file_as_text(path: &str) -> Result<String, Errors> {
    let mut text = String::new();

    let mut file = File::open(path).map_err(|err| {
        log::error!("Failed to open file: {}", err);
        Errors::FileInputError
    })?;

    file.read_to_string(&mut text).map_err(|err| {
        log::error!("Failed to read file: {}", err);
        Errors::FileInputError
    })?;

    Ok(text)
}

pub fn write_text_to_file(path: &str, text: &str) -> io::Result<()> {
    let mut file = File::create(path)?;

    file.write_all(text.as_bytes())?;

    Ok(())
}

pub fn append_to_filename(path: &str, suffix: &str) -> Result<String, Errors> {
    let path = Path::new(path);

    let stem = path.file_stem()
        .ok_or(Errors::PathConversionError)?
        .to_string_lossy();

    let extension = path.extension()
        .map_or(String::new(), |ext| ext.to_string_lossy().to_string());

    let new_filename = if extension.is_empty() {
        format!("{}{}", stem, suffix)
    } else {
        format!("{}{}.{}", stem, suffix, extension)
    };

    let binding = path.with_file_name(new_filename);
    let new_path = binding
        .to_str()
        .ok_or(Errors::PathConversionError)?;

    Ok(new_path.to_string())
}

impl From<CmdError> for Errors {
    fn from(err: CmdError) -> Errors {
        Errors::FetchUrlError(format!("Fantoccini command error: {:?}", err))
    }
}

pub async fn fetch_url_as_text(url: &str) -> Result<String, Errors> {
    let mut caps: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    caps.insert("browserName".to_string(), serde_json::Value::String("chrome".to_string()));

    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.107 Safari/537.36";
    caps.insert(
        "goog:chromeOptions".to_string(),
        serde_json::json!({
            "args": [
                "--headless",
                "--disable-gpu",
                "--window-size=1920,1080",
                &format!("--user-agent={}", user_agent)
            ]
        }),
    );
    caps.insert("acceptInsecureCerts".to_string(), serde_json::Value::Bool(true));

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:9515")
        .await
        .map_err(|err| Errors::FetchUrlError(
            format!("Failed to connect to WebDriver: {:?}", err)
        ))?;

    client.goto(url).await?;

    let html: String = client.find(Locator::Css("html")).await?.html(true).await?;

    client.close().await?;

    Ok(html)
}
