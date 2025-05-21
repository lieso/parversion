use serde::{Serialize, Deserialize};
use std::collections::{HashSet};

use crate::prelude::*;
use crate::transformation::{
    XMLElementTransformation,
    HashTransformation,
    Runtime
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<Hash>,
    pub xml_element_transformation: Option<XMLElementTransformation>,
    pub hash_transformation: Option<HashTransformation>,
    pub meaningful_fields: Option<Vec<String>>,
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

    pub async fn create_profile(
        features: &HashSet<Hash>
    ) -> Result<Profile, Errors> {
        log::trace!("In create_profile");

        let profile = Profile {
            id: ID::new(),
            description: "Placeholder description".to_string(),
            features: features.clone(),
            xml_element_transformation: Some(XMLElementTransformation {
                id: ID::new(),
                description: "XML element transformation applied during document preprocessing that blacklists certain elements or attributes to reduce document size and improve interpretation.".to_string(),
                runtime: Runtime::QuickJS,
                infix: r#"
const BLACKLISTED_ATTRIBUTES = {style:1, bgcolor:1, border:1, cellpadding:1, cellspacing:1, width:1, height:1, rows:1, cols:1, wrap:1, "aria-hidden":1, size:1, op:1, lang:1,
olspan:1, rel:1};
const BLACKLISTED_ELEMENTS = {script:1,meta:1,link:1,iframe:1,svg:1,style:1,noscript:1};
if (BLACKLISTED_ELEMENTS[element]) element = null;
attributes = Object.keys(attributes)
  .filter(item => !BLACKLISTED_ATTRIBUTES[item])
  .reduce((acc, key) => {
      acc[key] = attributes[key];
      return acc;
  }, {});"#.to_string(),
            }),
            hash_transformation: Some(HashTransformation {
                id: ID::new(),
                description: "Determines the set of input strings from a node to use in identity hash calculation".to_string(),
                runtime: Runtime::QuickJS,
                infix: r#"
let hasherItems = Object.keys(fields).sort()"#.to_string(),
            }),
            meaningful_fields: Some(vec!["text".to_string(), "href".to_string(), "title".to_string()]),
        };

        Ok(profile)
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
