use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::graph_node::Graph;
use crate::normal_context::NormalContext;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct NormalMetaContext {
    pub contexts: HashMap<ID, Arc<NormalContext>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<NormalContext>>,
}

impl NormalMetaContext {
    // Warning: this assumes both NormalMetaContexts are already part of the same graph
    pub fn merge(mut self, other: NormalMetaContext) -> Result<NormalMetaContext, Errors> {
        self.contexts.extend(other.contexts);
        self.contexts_lookup.extend(other.contexts_lookup);
        Ok(self)
    }
}
