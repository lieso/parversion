use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::prelude::*;
use crate::transformation::{HashTransformation, Runtime, XMLElementTransformation};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: ID,
    pub description: String,
    pub features: HashSet<Hash>,
    pub preprocess_transformations: Vec<DocumentNodeTransformation>,
    pub hash_transformation: HashTransformation,
    pub fields_transformation: DocumentNodeFieldsTransformation,
}
