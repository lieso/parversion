use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::NetworkTranslationTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslationNetwork {
    pub id: ID,
    pub source_lineage: Lineage,
    pub target_lineage: Lineage,
    pub transformation: NetworkTranslationTransformation,
}
