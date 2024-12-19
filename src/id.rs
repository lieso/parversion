use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
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

impl PartialEq for ID {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for ID {}
