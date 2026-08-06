use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::HashSet;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisClusterMetadata {
    pub prompts: Vec<Hash>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkRelationshipType {
    Combine,
    Equal { canonical: Hash },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationship {
    pub id: ID,
    pub from: Hash,
    pub to: Hash,
    pub relationship: NetworkRelationshipType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisCluster {
    pub id: ID,
    pub networks: HashSet<Hash>,
    pub relationships: Vec<NetworkRelationship>,
    pub metadata: BasisClusterMetadata,
}
