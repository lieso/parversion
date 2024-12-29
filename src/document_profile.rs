use serde::{Serialize, Deserialize};
use std::collections::{HashSet};

use crate::prelude::*;
use crate::transformation::DocumentTransformation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentProfile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<u64>,
    pub transformations: Vec<DocumentTransformation>,
}

impl DocumentProfile {
    pub fn get_similar_profile(
        profiles: &Vec<DocumentProfile>,
        features: HashSet<Hash>
    ) -> Option<DocumentProfile> {
        profiles.iter().find(|profile| {
            let similarity = jaccard_similarity(features, &profile.features);

            log::debug!("similarity: {}", similarity);

            similarity > 0.8
        })
    }
}

fn jaccard_similarity(set_a: &HashSet<u64>, set_b: &HashSet<u64>) -> f64 {
    let intersection: HashSet<_> = set_a.intersection(set_b).collect();
    let union: HashSet<_> = set_a.union(set_b).collect();

    if union.is_empty() {
        return 1.0;
    }

    intersection.len() as f64 / union.len() as f64
}
