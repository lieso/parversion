use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Timestamp {
    #[serde(with = "chrono::serde::ts_seconds")]
    time: DateTime<Utc>,
}

impl Timestamp {
    pub fn now() -> Self {
        Timestamp {
            time: Utc::now(),
        }
    }
}
