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

    let basis_networks = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
            })?
    };

    unimplemented!()
}
