use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkRelationship {
    pub id: ID,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
}
