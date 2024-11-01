use std::env;
use dotenv::dotenv;

#[derive(PartialEq)]
enum Environment {
    Local,
    Production,
}

pub fn is_local() -> bool {
    dotenv().ok();
    let value = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());
    Environment::from_str(&value) == Environment::Local
}

impl Environment {
    fn from_str(env: &str) -> Self {
        match env {
            "local" => Environment::Local,
            _ => Environment::Production,
        }
    }
}
