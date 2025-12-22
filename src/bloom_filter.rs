use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BloomFilter {
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
        usize::from_str_radix(&value.to_string().unwrap(), 16).unwrap() % self.size
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
