use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
pub struct ID {
    value: String
}

impl ID {
    pub fn new() -> Self {
        ID {
            value: Uuid::new_v4().to_string()
        }
    }
    
    pub fn from_str(value: &str) -> Self {
        ID {
            value: value.to_string()
        }
    }

    pub fn to_string(&self) -> String {
        self.value.clone()
    }
}

impl PartialEq for ID {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for ID {}
