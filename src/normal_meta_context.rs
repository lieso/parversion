use std::sync::Arc;
use std::collections::{HashMap};

use crate::prelude::*;
use crate::graph_node::Graph;
use crate::normal_context::NormalContext;

#[derive(Clone, Debug)]
pub struct NormalMetaContext {
    pub contexts: HashMap<ID, Arc<NormalContext>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<NormalContext>>,
}
