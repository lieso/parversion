use sled::Db;
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::prelude::*;
use crate::config::{CONFIG};

static DB: Lazy<Arc<Db>> = Lazy::new(|| {
    let debug_dir = &read_lock!(CONFIG).dev.debug_dir;
    let db = sled::open(format!("{}/cache", debug_dir)).expect("Could not open cache");
    Arc::new(db)
});

pub struct Cache {}

impl Cache {
    pub async fn get_or_set_cache<F, Fut>(hash: Hash, fetch_data: F) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let hash_str = &hash.to_string().unwrap();
        if let Some(cached_response) = Self::get_cached_response(hash_str) {
            log::info!("Cache hit!");
            Some(cached_response)
        } else {
            log::info!("Cache miss!");
            if let Some(response) = fetch_data().await {
                Self::set_cached_response(hash_str, &response);
                Some(response)
            } else {
                None
            }
        }
    }

    fn get_cached_response(key: &str) -> Option<String> {
        let db = DB.clone();
        match db.get(key).expect("Could not get value from cache") {
            Some(data) => Some(String::from_utf8(data.to_vec()).expect("Could not deserialize data")),
            None => None,
        }
    }

    fn set_cached_response(key: &str, value: &str) {
        let db = DB.clone();
        db.insert(key, value.to_string().into_bytes()).expect("Could not store value in cache");
    }
}
