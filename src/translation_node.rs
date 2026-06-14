use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::FieldTranslationTransformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslationNode {
    pub id: ID,
    pub source_lineage: Lineage,
    pub target_lineage: Lineage,
    pub transformations: Vec<FieldTranslationTransformation>
}
