use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::CanonicalizationTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub hash: Hash,
    pub transformation: CanonicalizationTransformation
}
