use std::sync::{Arc, RwLock};

use crate::ast::program_to_functions;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::function_analysis::functions_to_operations;
use crate::meta_context::MetaContext;
use crate::network_analysis::{get_classification, get_basis_networks, get_network_relationships};
use crate::node_analysis::get_basis_nodes;
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;

pub async fn normalize<P: Provider>(
    provider: Arc<P>,
    document: Document,
    options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize");

    let meta_context = normalize_to_classification(
        Arc::clone(&provider),
        document,
        options,
        metadata,
        execution_context.clone(),
    )
    .await?;

    let stage = execution_context.enter_stage("Node analysis");

    log::info!("Getting basis nodes");
    let basis_nodes =
        get_basis_nodes(
            Arc::clone(&provider),
            meta_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_nodes(basis_nodes);
    }

    stage.finish();
    let stage = execution_context.enter_stage("Network analysis");

    log::info!("Generating basis networks");
    let basis_networks =
        get_basis_networks(
            Arc::clone(&provider),
            meta_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_networks(basis_networks);
    }

    stage.finish();
    let stage = execution_context.enter_stage("Network relationships");

    log::info!("Generating network relationships");

    let basis_graph =
        get_network_relationships(
            Arc::clone(&provider),
            Arc::clone(&meta_context),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_basis_graph(basis_graph);
    }

    stage.finish();

    {
        let normalized = Document::from_basis_transformations(Arc::clone(&meta_context))?;
        let result = format!("{}", normalized.to_string(&None));
        log::debug!(
            "\n\n\
        =======================================================\n\
        =============   NORMALIZED DOCUMENT START   =================\n\
        =======================================================\n\
        {}
        =======================================================\n\
        =============    NORMALIZED DOCUMENT END    =================\n\
        =======================================================\n\n",
            result
        );
    }

    unimplemented!();

    Ok(meta_context)
}

pub async fn normalize_to_classification<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_to_classification");
    let _ = execution_context;

    let stage = execution_context.enter_stage("Document preprocessing and classification");

    let meta_context = Arc::new(RwLock::new(MetaContext::new()));

    {
        let mut lock = write_lock!(meta_context);
        lock.add_document_version(DocumentVersion::InputDocument, document.clone());
    }

    // ******************************************************************************************************

    if metadata.document_type == Some(DocumentType::JavaScript) {
        let functions = program_to_functions(document.data.clone());

        for function in functions.iter() {
            log::debug!("hash: {}", function.hash);
            log::debug!("{}\n", function.code);
        }

        log::debug!("function count: {}", functions.len());

        {
            let mut lock = write_lock!(meta_context);
            lock.update_functions(functions);
        }

        let _something =
            functions_to_operations(Arc::clone(&provider), meta_context.clone()).await?;

        unimplemented!();
    }

    // ******************************************************************************************************

    log::info!("Performing document analysis");
    let profile = document.perform_analysis(Arc::clone(&provider)).await?;
    let profile = Arc::new(profile);

    {
        let mut lock = write_lock!(meta_context);
        lock.update_profile(profile);
    }

    log::info!("Traversing document");
    let (contexts, graph_root) = document.get_contexts(meta_context.clone(), metadata)?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_data_structures(contexts, graph_root);
    }

    log::info!("Getting classification");
    let classification =
        get_classification(
            Arc::clone(&provider),
            meta_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_classification(classification);
    }

    stage.finish();

    Ok(meta_context)
}

pub async fn normalize_document_to_classification<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_document_to_classification");

    normalize_to_classification(
        Arc::clone(&provider),
        document,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_document<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::trace!("In normalize_document");

    let meta_context =
        normalize(Arc::clone(&provider), document, _options, metadata, execution_context).await?;

    let normalized_document = Document::from_basis_transformations(Arc::clone(&meta_context))?;

    Ok(Package {
        document: normalized_document,
        mutations: Vec::new(),
    })
}

pub async fn normalize_document_to_string<P: Provider>(
    provider: Arc<P>,
    document: Document,
    _options: &Options,
    metadata: &Metadata,
    document_format: &Option<DocumentFormat>,
    execution_context: Arc<ExecutionContext>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_string");

    let package = normalize_document(
        Arc::clone(&provider),
        document,
        _options,
        metadata,
        execution_context,
    )
    .await?;

    Ok(package.to_string(document_format))
}

pub async fn normalize_text<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, _options, metadata)?;

    normalize(
        Arc::clone(&provider),
        document,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_text_to_classification<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_text_to_classification");

    let document = Document::from_string(text, _options, metadata)?;

    normalize_to_classification(
        Arc::clone(&provider),
        document,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_text_to_document<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let meta_context =
        normalize_text(Arc::clone(&provider), text, _options, metadata, execution_context).await?;

    let normalized_document = Document::from_basis_transformations(Arc::clone(&meta_context));

    normalized_document
}

pub async fn normalize_file<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    normalize_text(
        Arc::clone(&provider),
        text,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_file_to_classification<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In normalize_file_to_classification");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    normalize_text_to_classification(
        Arc::clone(&provider),
        text,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_file_to_document<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let meta_context =
        normalize_file(Arc::clone(&provider), path, _options, metadata, execution_context).await?;

    let normalized_document = Document::from_basis_transformations(Arc::clone(&meta_context));

    normalized_document
}

pub async fn normalize_file_to_string<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    document_format: &Option<DocumentFormat>,
    execution_context: Arc<ExecutionContext>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_string");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(
        Arc::clone(&provider),
        path,
        _options,
        metadata,
        execution_context,
    )
    .await?;

    Ok(document.to_string(document_format))
}
