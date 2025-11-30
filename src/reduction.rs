use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document, DocumentType};
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::mutations::Mutations;

pub async fn reduce<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In reduce");

    unimplemented!()
}

pub async fn reduce_text_to_mutations<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_text_to_mutations");

    unimplemented!()
}

pub async fn reduce_url_to_mutations<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_url_to_mutations");

    unimplemented!()
}


pub async fn reduce_file_to_mutations<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_file_to_mutations");

    unimplemented!()
}


