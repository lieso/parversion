use std::sync::{Arc, RwLock};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::basis_field::BasisField;

pub async fn get_basis_fields<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisField>>, Errors> {
    log::trace!("In get_basis_fields");
    unimplemented!()
}
