use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{Context, ContextID};
use crate::graph_node::Graph;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct MetaContext {
    pub contexts: HashMap<ContextID, Arc<Context>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<Context>>,
}
