use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::document::{Document, DocumentType};
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::mutations::Mutations;
use crate::ast::program_to_functions;
use crate::package::Package;

pub async fn reduce<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In reduce");

    unimplemented!()
}

pub async fn reduce_text_to_package<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Package, Errors> {
    log::trace!("In reduce_text_to_package");


    let functions = program_to_functions(text);


    for function in functions.iter() {
        log::debug!("hash: {}", function.hash);
        log::debug!("{}\n", function.code);
    }

    log::debug!("functions: {}", functions.len());


    unimplemented!()
}

pub async fn reduce_url_to_package<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Package, Errors> {
    log::trace!("In reduce_url_to_package");

    unimplemented!()
}


pub async fn reduce_file_to_package<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Package, Errors> {
    log::trace!("In reduce_file_to_package");

    unimplemented!()
}
