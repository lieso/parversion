use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::FieldTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslationNode {
    pub id: ID,
    pub source_lineage: Lineage,
    pub target_lineage: Lineage,
    pub transformation: FieldTransformation
}
