use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document};
use crate::provider::Provider;
use crate::meta_context::MetaContext;

pub async fn reduce<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In reduce");

    unimplemented!()
}
