use serde::{Deserialize, Serialize};

use crate::network_relationship::NetworkRelationshipType;
use crate::prelude::*;
use crate::transformation::{
    CanonicalizationTransformation,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub hash: Hash,
    pub canonicalization: CanonicalizationTransformation,
    pub relationships: Option<Vec<(ID, ID, NetworkRelationshipType)>>,
}
