use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hash {
    items: Vec<_>,
    value: Option<String>,
}

impl Hash {
    pub fn from_str(s: &str) -> Self {
        Hash {
            item: Vec::new(),
            value: Some(s),
        }
    }

    pub fn from_items<T>(self, items: Vec<T>) -> Self {
        Hash {
            items: items.iter().map(item -> item.to_string()),
            value: None,
        }
    }
    
    pub fn push(self, item: T) -> self {
        self.items.push(item);
        self
    }

    pub fn sort(self) -> self {
        self.items.sort();
        self
    }

    pub fn finalize(self) -> self {
        let mut hasher = Sha256::new();
        hasher.update(self.items.join(""));

        let finalized = format!("{:x}", hasher.finalize());
        self.value = Some(finalized);
        self
    }

    pub fn to_string(self) -> String {
        self.value.unwrap()
    }
}
