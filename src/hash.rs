use sha2::{Sha256, Digest};

pub struct Hash {
    items: Vec<_>,
    value: Option<String>,
}

impl Hash {
    pub fn new() -> self {
        self.items = Vec::new();
        self
    }

    pub fn with_items(self, items: Vec<_>) -> self {
        self.items = items;
        self
    }
    
    pub fn push(self, item: T) -> self {
        self.item.push(item);
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
        value
    }
}
