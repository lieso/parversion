use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::Graph;
use crate::normalization_context::NormalizationContext;

pub struct Xslt {}

impl Xslt {
    pub fn new(stylesheet: &str) -> Result<Self, Errors> {
        unimplemented!()
    }

    pub fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Vec<Graph>, Errors> {
        unimplemented!()
    }
}
