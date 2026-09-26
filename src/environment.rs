use dotenv::dotenv;

pub fn get_env_variable(key: &str) -> String {
    std::env::var(key).unwrap().to_string()
}
