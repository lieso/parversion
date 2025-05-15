use dotenv::dotenv;

pub fn get_env_variable(key: &str) -> String {
    dotenv().ok();

    std::env::var(key).unwrap().to_string()
}
