use serde::Deserialize;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub target_node_adjacent_xml_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_contents)?;
        Ok(config)
    }
}

lazy_static! {
    pub static ref CONFIG: Mutex<Config> = Mutex::new(
        Config::load_from_file("settings.toml").expect("Failed to load configuration"),
    );
}
