use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::mutation::Mutation;

pub async fn functions_to_mutations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<Hash, Arc<Mutation>>, Errors> {
    log::trace!("In functions_to_mutations");

    unimplemented!()
}
