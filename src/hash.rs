use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as StdHash, Hasher as StdHasher};
use std::fmt;
use std::fmt::Debug;

pub trait HashStrategy {
    fn hash(data: &[u8]) -> String;
}

#[derive(Debug, Clone)]
pub struct Sha256Strategy;
impl HashStrategy for Sha256Strategy {
    fn hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

// DefaultHasher (non-cryptographic) strategy
#[derive(Debug, Clone)]
pub struct DefaultHasherStrategy;
impl HashStrategy for DefaultHasherStrategy {
    fn hash(data: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hasher<T: HashStrategy> {
    items: Vec<String>,
    value: Option<String>,
    _strategy: std::marker::PhantomData<T>,
}

impl<T: HashStrategy> Hasher<T> {
    pub fn new() -> Self {
        Hasher {
            items: Vec::new(),
            value: None,
            _strategy: std::marker::PhantomData,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Hasher {
            items: Vec::new(),
            value: Some(T::hash(s.as_bytes())),
            _strategy: std::marker::PhantomData,
        }
    }

    pub fn from_items<U: ToString>(items: Vec<U>) -> Self {
        let string_items = items.into_iter().map(|item| item.to_string()).collect();
        Hasher {
            items: string_items,
            value: None,
            _strategy: std::marker::PhantomData,
        }
    }

    pub fn push<U: ToString>(&mut self, item: U) -> &mut Self {
        self.items.push(item.to_string());
        self.value = None;
        self
    }

    pub fn sort(&mut self) -> &mut Self {
        self.items.sort();
        self
    }

    pub fn finalize(&mut self) -> &mut Self {
        let concatenated = self.items.join("").into_bytes();
        self.value = Some(T::hash(&concatenated));
        self
    }

    pub fn is_unfinalized(&self) -> bool {
        self.value.is_none()
    }

    pub fn to_string(&self) -> Option<String> {
        self.value.clone()
    }
    
    pub fn clear_items(&mut self) -> &mut Self {
        self.items.clear();
        self
    }
}

impl<T: HashStrategy> PartialEq for Hasher<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: HashStrategy> Eq for Hasher<T> {}

impl<T: HashStrategy> fmt::Display for Hasher<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{}", value),
            None => write!(f, "<uncomputed hash>"),
        }
    }
}

pub type Hash = Hasher<Sha256Strategy>;
pub type FastHash = Hasher<DefaultHasherStrategy>;

