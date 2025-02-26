use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use std::sync::RwLock;
use std::path::Path;
use std::fs;
use std::io::Write;
use std::env;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum LlmProvider {
    OpenAI,
    Anthropic,
    Groq
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    pub llm_provider: LlmProvider,
    pub max_concurrency: usize,
    pub example_snippet_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DevConfig {
    pub debug_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
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
        Config {
            llm: LlmConfig {
                llm_provider: LlmProvider::OpenAI,
                max_concurrency: 1,
                example_snippet_count: 3,
            },
            dev: DevConfig {
                debug_dir: get_default_debug_dir(),
            }
        }
    }

    fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml = toml::to_string(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(toml.as_bytes())?;
        Ok(())
    }

    fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_contents)?;

        Ok(config)
    }

    fn load_or_create_default(path: &str) -> Config {
        if Path::new(path).exists() {
            Config::load_from_file(path).expect("Failed to load configuration")
        } else {
            let default_config = Config::default();
            default_config.save_to_file(path).expect("Failed to save default configutation");
            default_config
        }
    }
}

lazy_static! {
    pub static ref CONFIG: RwLock<Config> = RwLock::new(
        Config::load_or_create_default("settings.toml"),
    );
}
