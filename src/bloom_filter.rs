use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BloomFilter {
    size: usize,
    bits: Vec<bool>,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        BloomFilter {
            size,
            bits: vec![false; size],
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
            self.bits[index] = true;
        }
    }

    pub fn contains(&self, hash: &Hash) -> bool {
        for i in 0..self.num_hashes {
            let index = self.hash(hash, i as u64);
            if !self.bits[index] {
                return false;
            }
        }

        true
    }
}
