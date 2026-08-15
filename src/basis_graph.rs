use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
}
