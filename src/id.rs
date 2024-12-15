use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ID {
    value: String
}

impl ID {
    pub fn new() -> self {
        ID {
            value: Uuid::new_v4().to_string()
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}", self.value);
    }

}
