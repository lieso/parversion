use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentProfile {
    pub id: ID,
    pub features: Vec<f64>,
}
