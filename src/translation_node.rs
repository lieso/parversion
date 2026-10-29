use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::transformation::Transformation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslationNodeMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranslationNode {
    pub id: ID,
    pub transformations: Vec<Transformation>,
}

