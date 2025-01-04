use serde::{Serialize, Deserialize};
use std::collections::{HashSet};

use crate::prelude::*;
use crate::transformation::{
    DocumentTransformation,
    HashTransformation
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<Hash>,
    pub document_transformations: Option<Vec<DocumentTransformation>>,
    pub hash_transformation: Option<HashTransformation>
}

impl Profile {
    pub fn get_similar_profile(
        profiles: &Vec<Profile>,
        features: &HashSet<Hash>
    ) -> Option<Profile> {
        profiles.iter()
            .find(|profile| {
                let similarity = jaccard_similarity(features, &profile.features);

                log::debug!("similarity: {}", similarity);

                similarity > 0.8
            })
            .map(|profile| profile.clone())
    }
}

fn jaccard_similarity(set_a: &HashSet<Hash>, set_b: &HashSet<Hash>) -> f64 {
    let intersection: HashSet<_> = set_a.intersection(set_b).collect();
    let union: HashSet<_> = set_a.union(set_b).collect();

    if union.is_empty() {
        return 1.0;
    }

    intersection.len() as f64 / union.len() as f64
}
