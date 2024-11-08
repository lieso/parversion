use serde::Deserialize;
use lazy_static::lazy_static;
use std::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct LlmConfigDataStructureInterpretation {
    pub enabled: bool,
    pub target_node_adjacent_xml_length: usize,
    pub target_node_examples_max_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub target_node_adjacent_xml_length: usize,
    pub target_node_examples_max_count: usize,
    pub data_structure_interpretation: LlmConfigDataStructureInterpretation,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_contents)?;

        if config.llm.target_node_examples_max_count < 1 {
            panic!("It makes no sense for target_node_examples_max_count to be less than 1");
        }

        Ok(config)
    }
}

lazy_static! {
    pub static ref CONFIG: RwLock<Config> = RwLock::new(
        Config::load_from_file("settings.toml").expect("Failed to load configuration"),
    );
}
