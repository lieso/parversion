use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{self, Visitor};
use sha2::{Sha256, Digest};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Hash {
    items: Option<Vec<String>>,
    value: Option<String>,
}

impl Hash {
    pub fn new() -> Self {
        Hash {
            items: Some(Vec::new()),
            value: None,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Hash {
            items: Some(Vec::new()),
            value: Some(Self::hash(s.as_bytes())),
        }
    }

    pub fn from_items<U: ToString>(items: Vec<U>) -> Self {
        let string_items = items.into_iter().map(|item| item.to_string()).collect();
        Hash {
            items: Some(string_items),
            value: None,
        }
    }

    pub fn push<U: ToString>(&mut self, item: U) -> &mut Self {
        if self.items.is_none() {
            self.items = Some(Vec::new());
        }
        if let Some(ref mut items) = self.items {
            items.push(item.to_string());
            self.value = None;
        }
        self
    }

    pub fn sort(&mut self) -> &mut Self {
        if let Some(ref mut items) = self.items {
            items.sort();
        }
        self
    }

    pub fn finalize(&mut self) -> &mut Self {
        if let Some(ref items) = self.items {
            let concatenated = items.join("").into_bytes();
            self.value = Some(Self::hash(&concatenated));
        }
        self
    }

    pub fn is_unfinalized(&self) -> bool {
        self.value.is_none()
    }

    pub fn to_string(&self) -> Option<String> {
        self.value.clone()
    }

    pub fn clear_items(&mut self) -> &mut Self {
        self.items = Some(Vec::new());
        self
    }

    fn hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

impl PartialEq for Hash {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
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

impl std::hash::Hash for Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(ref value) = self.value {
            value.hash(state);
        }
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value {
            Some(value) => serializer.serialize_str(value),
            None => Err(serde::ser::Error::custom("Hash value is missing")),
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HashVisitor;

        impl<'de> Visitor<'de> for HashVisitor {
            type Value = Hash;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid hash string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Hash, E>
            where
                E: de::Error,
            {
                Ok(Hash {
                    items: None,
                    value: Some(value.to_string()),
                })
            }
        }

        deserializer.deserialize_str(HashVisitor)
    }
}
