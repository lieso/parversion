use std::sync::Arc;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata};
use crate::classification::Classification;

pub async fn classify<R: Reasoner>(
    reasoner: &R,
    meta_context: Arc<MetaContext>
) -> Result<(Classification, ReasonerMetadata), Errors> {

    let context_string = meta_context.generate_context_string()?;

    todo!()
}
