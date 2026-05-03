use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    pub max_concurrency: usize,
    pub example_snippet_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DevConfig {
    #[serde(default = "get_default_debug_dir")]
    pub debug_dir: String,
}

impl Default for DevConfig {
    fn default() -> Self {
        DevConfig {
            debug_dir: get_default_debug_dir(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    #[serde(default)]
    pub dev: DevConfig,
}

fn get_default_debug_dir() -> String {
    env::current_dir()
        .expect("Could not get current working directory")
        .to_str()
        .unwrap_or("/dev/null")
        .to_string()
}

impl Config {
    fn default() -> Self {
        let config = Config {
            llm: LlmConfig {
                max_concurrency: 1,
                example_snippet_count: 3,
            },
            dev: DevConfig::default(),
        };

        config
    }

    fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_contents)?;

        Ok(config)
    }

    fn load_or_create_default(path: &str) -> Config {
        if Path::new(path).exists() {
            Config::load_from_file(path).unwrap_or_else(|e| {
                panic!("Failed to load config from settings.toml: {}", e);
            })
        } else {
            Config::default()
        }
    }
}

lazy_static! {
    pub static ref CONFIG: RwLock<Config> =
        RwLock::new(Config::load_or_create_default("settings.toml"),);
}
