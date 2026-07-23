use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::basis_graph::BasisGraph;

pub async fn generate_basis_graph<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<BasisGraph, Errors> {
    log::trace!("In get_network_relationships");

    unimplemented!()
}
