use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::document::{Document, DocumentRole};
use crate::document_format::DocumentFormat;
use crate::normalization;
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::translation_context::TranslationContext;

pub async fn translate<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In translate");

    let start = Instant::now();
    let stage = execution_context.enter_stage("Initialization");

    let _translation_context = init_translation_context(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        source,
        target,
        options,
        execution_context.clone(),
    )
    .await?;

    stage.finish();
    let elapsed = start.elapsed();
    log::info!("init_translation_context: {:.2?}", elapsed);

    unimplemented!()
}

pub async fn translate_text_to_document<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In translate_text_to_document");

    let translation_context = translate_text(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        source,
        target,
        options,
        execution_context,
    )
    .await?;

    let translated_document =
        Document::from_translation(Arc::clone(&translation_context), document_format)?;

    Ok(translated_document)
}

pub async fn translate_text<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In translate_text");

    let source_document = Document::from_string(source.0, options, source.1)?;

    let start = Instant::now();
    let stage = execution_context.enter_stage("Preinitialization");

    let target_document = {
        match target.1.role {
            DocumentRole::Instance => Document::from_string(target.0, options, target.1)?,
            DocumentRole::Schema => {
                Document::from_schema_string(
                    Arc::clone(&provider),
                    Arc::clone(&reasoner),
                    target.0,
                    options,
                    target.1,
                    &stage,
                )
                .await?
            }
        }
    };

    stage.finish();
    let elapsed = start.elapsed();
    log::info!("preinitialization: {:.2?}", elapsed);

    let translation_context = translate(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        source_document,
        target_document,
        options,
        execution_context.clone(),
    )
    .await?;

    Ok(translation_context)
}

pub async fn translate_text_to_package<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    source: (String, &Metadata),
    target: (String, &Metadata),
    options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::trace!("In translate_text_to_package");

    let translated_document = translate_text_to_document(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        source,
        target,
        options,
        document_format,
        execution_context,
    )
    .await?;

    Ok(Package {
        documents: vec![translated_document],
    })
}

async fn init_translation_context<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    source: Document,
    target: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<TranslationContext>>, Errors> {
    log::trace!("In init_translation_context");

    let (source, target) = futures::try_join!(
        normalization::normalize(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            source,
            options,
            execution_context.clone(),
        ),
        normalization::normalize(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            target,
            options,
            execution_context.clone(),
        )
    )?;

    Ok(Arc::new(RwLock::new(TranslationContext::new(
        source, target,
    ))))
}
