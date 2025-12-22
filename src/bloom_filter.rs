use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::fmt;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BloomFilter {
    size: usize,
    bits: String,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        let bits = "0".repeat(size);
        BloomFilter {
            size,
            bits,
            num_hashes,
        }
    }

    fn hash(&self, value: &Hash, seed: u64) -> usize {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", &value.to_string().unwrap(), seed));
        let hash_value = hasher.finalize();
        let hash_bytes = &hash_value[..];
        let hash_num = usize::from_le_bytes(hash_bytes[0..8].try_into().unwrap());
        hash_num % self.size
    }

    pub fn add(&mut self, hash: &Hash) {
        for i in 0..self.num_hashes {
            let index = self.hash(hash, i as u64);
            let mut chars: Vec<char> = self.bits.chars().collect();
            if index < chars.len() {
                chars[index] = '1';
                self.bits = chars.iter().collect();
            }
        }
    }

    pub fn contains(&self, hash: &Hash) -> bool {
        for i in 0..self.num_hashes {
            let index = self.hash(hash, i as u64);
            if self.bits.chars().nth(index) == Some('0') {
                return false;
            }
        }

        true
    }
}
