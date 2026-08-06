use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prelude::*;
use crate::basis_cluster::BasisCluster;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub basis_clusters: Vec<BasisCluster>,
}
