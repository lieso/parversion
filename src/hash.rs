use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::fmt::Debug;
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hash {
    items: Vec<String>,
    value: Option<String>,
}

impl Hash {
    pub fn new() -> Self {
        Hash {
            items: Vec::new(),
            value: None,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Hash {
            items: Vec::new(),
            value: Some(Self::hash_str(s)),
        }
    }

    pub fn from_items<T: ToString>(items: Vec<T>) -> Self {
        let string_items = items.into_iter().map(|item|
item.to_string()).collect::<Vec<String>>();
        Hash {
            items: string_items,
            value: None,
        }
    }

    pub fn push<T: ToString>(&mut self, item: T) -> &mut Self {
        self.items.push(item.to_string());
        self.value = None; // Invalidate previous hash
        self
    }

    pub fn sort(&mut self) -> &mut Self {
        self.items.sort();
        self
    }

    pub fn finalize(&mut self) -> &mut Self {
        let mut hasher = Sha256::new();
        let concatenated = self.items.join("");
        hasher.update(concatenated);

        let finalized = format!("{:x}", hasher.finalize());
        self.value = Some(finalized);
        self
    }

    pub fn is_unfinalized(&self) -> bool {
        self.value.is_none()
    }

    pub fn to_string(&self) -> Option<String> {
        self.value.clone()
    }

    pub fn hash_str(s: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(s);
        format!("{:x}", hasher.finalize())
    }
}

impl PartialEq for Hash {
    fn eq(&self, other: &Self) -> bool {
        if let Some(value) = &self.value {
            if let Some(other_value) = &other.value {
                return *value == *other_value;
            }
        }

        false
    }
}

impl Eq for Hash {}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{}", value),
            None => write!(f, "<uncomputed hash>"),
        }
    }
}

