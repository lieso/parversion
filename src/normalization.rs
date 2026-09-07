use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::normalization_context::NormalizationContext;
use crate::field_analysis::generate_basis_fields;
use crate::group_analysis::{generate_basis_groups, resolve_context_groups};
use crate::node_analysis::{generate_basis_nodes};
use crate::network_analysis::{
    get_classification,
    generate_basis_networks
};
use crate::reports::{
    report_basis_groups,
    report_basis_fields,
    report_basis_nodes,
    report_basis_networks,
};
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::graph_node::Graph;
use crate::graph_node::GraphNode;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::basis_group::BasisGroup;
use crate::normal_context::NormalContext;
use crate::data_node::{DataNode, DataNodeFields};
use crate::classification::Classification;
use crate::normal_meta_context::NormalMetaContext;

pub async fn normalize<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    log::trace!("In normalize");

    let start = Instant::now();
    let stage = execution_context.enter_stage("Initialization");

    let normalization_context = init_normalization_context(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        document,
        options,
    )
    .await?;

    stage.finish();
    let elapsed = start.elapsed();
    log::info!("init_normalization_context: {:.2?}", elapsed);

    let start = Instant::now();
    let stage = execution_context.enter_stage("Document classification");

    let classification =
        get_classification(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            normalization_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_classification(classification);
    }

    stage.finish();
    let elapsed = start.elapsed();
    log::info!("get_classification: {:.2?}", elapsed);

    let start = Instant::now();
    let stage = execution_context.enter_stage("Field analysis");

    let basis_fields =
        generate_basis_fields(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            Arc::clone(&normalization_context),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_fields(basis_fields);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_fields: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_fields(Arc::clone(&provider), Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Group analysis");

    let basis_groups =
        generate_basis_groups(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            Arc::clone(&normalization_context),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_groups(basis_groups);
    }

    let (context_groups, context_to_group) = resolve_context_groups(
        Arc::clone(&normalization_context)
    )?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_context_groups(context_groups, context_to_group);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_groups: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_groups(Arc::clone(&provider), Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Node analysis");

    log::info!("Getting basis nodes");
    let (basis_nodes, basis_node_contexts) =
        generate_basis_nodes(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            normalization_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_nodes(basis_nodes, basis_node_contexts);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_nodes: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_nodes(Arc::clone(&provider), Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Network analysis");

    log::info!("Generating basis networks");
    let (basis_networks,) =
        generate_basis_networks(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            normalization_context.clone(),
            &options,
            &stage,
        )
        .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_networks(basis_networks);
    }

    let elapsed = start.elapsed();
    log::info!("get_basis_networks: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_networks(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Building normalized graph");

    let normalized = build_normalized_graph(
        Arc::clone(&provider),
        Arc::clone(&normalization_context),
        &options,
    )?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_normalized_graph(normalized);
    }

    let elapsed = start.elapsed();
    log::info!("build_normalized_graph: {:.2?}", elapsed);

    stage.finish();

    Ok(normalization_context)
}

async fn normalize_html<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    options: &Options,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let mut document = document;

    log::info!("Traversing document");
    let meta_context = document.to_meta_context()?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_meta_context(meta_context);
    }

    Ok(())
}

pub async fn normalize_document<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    _options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Package, Errors> {
    log::trace!("In normalize_document");

    let normalization_context =
        normalize(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            document,
            _options,
            execution_context
        ).await?;

    let normalized_document = Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)?;

    Ok(Package {
        document: normalized_document,
        mutations: Vec::new(),
    })
}

pub async fn normalize_document_to_string<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    _options: &Options,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<String, Errors> {
    log::trace!("In normalize_document_to_string");

    let package = normalize_document(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        document,
        _options,
        document_format,
        execution_context,
    )
    .await?;

    Ok(package.to_string())
}

pub async fn normalize_text<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, _options, metadata)?;

    normalize(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        document,
        _options,
        execution_context,
    )
    .await
}

pub async fn normalize_text_to_document<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_text_to_document");

    let normalization_context =
        normalize_text(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            text,
            _options,
            metadata,
            execution_context
        ).await?;

    Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)
}

pub async fn normalize_file<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file path: {}", path);

    let text = get_file_as_text(path).map_err(|err| {
        log::error!("Failed to get file as text: {:?}", err);
        Errors::FileInputError
    })?;

    normalize_text(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        text,
        _options,
        metadata,
        execution_context,
    )
    .await
}

pub async fn normalize_file_to_document<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<Document, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let normalization_context =
        normalize_file(
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            path,
            _options,
            metadata,
            execution_context
        ).await?;

    Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)
}

pub async fn normalize_file_to_string<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    document_format: &DocumentFormat,
    execution_context: Arc<ExecutionContext>,
) -> Result<String, Errors> {
    log::trace!("In normalize_file_to_string");
    log::debug!("file path: {}", path);

    let document = normalize_file_to_document(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        path,
        _options,
        metadata,
        document_format,
        execution_context,
    )
    .await?;

    Ok(document.to_string())
}

async fn init_normalization_context<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    options: &Options,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    log::trace!("In init_normalization_context");

    let normalization_context = Arc::new(RwLock::new(NormalizationContext::new()));

    {
        let mut lock = write_lock!(normalization_context);
        lock.add_document_version(DocumentVersion::InputDocument, document.clone());
    }

    match document.document_type {
        DocumentType::Html => {
            normalize_html(
                Arc::clone(&provider),
                Arc::clone(&reasoner),
                document,
                options,
                normalization_context.clone(),
            )
            .await?;
        }
        DocumentType::Json => {
            unimplemented!();
        }
        DocumentType::PlainText => {
            unimplemented!();
        }
        DocumentType::JavaScript => {
            unimplemented!();
        }
        DocumentType::Xml => {
            unimplemented!();
        }
    }

    Ok(normalization_context)
}

fn build_normalized_graph<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options
) -> Result<NormalMetaContext, Errors> {
    log::trace!("In build_normalized_graph");

    let classification: Arc<Classification> = {
        let lock = read_lock!(normalization_context);
        lock.classification.clone().ok_or(Errors::ClassificationNotFound)?
    };

    let root = Arc::new(RwLock::new(GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        description: String::from("placeholder description"),
        hash: Hash::new(),
        subgraph_hash: Hash::new(),
        lineage: Lineage::new(),
        children: Vec::new(),
    }));

    let basis_networks = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
            })?
            .clone()
    };

    let mut normalized = basis_networks
        .values()
        .try_fold(None, |acc, basis_network| -> Result<Option<NormalMetaContext>, Errors> {
            let normal_meta_context = basis_network.apply(
                Arc::clone(&normalization_context),
                Arc::clone(&root)
            )?;

            if let Some(result) = acc {
                Ok(Some(result.merge(normal_meta_context)?))
            } else {
                Ok(Some(normal_meta_context))
            }
        })?
        .unwrap();

    let root_context = Arc::new(NormalContext {
        id: ID::new(),
        network_name: Some(classification.name.clone()),
        network_description: Some(classification.description.clone()),
        graph_node: Arc::clone(&root),
        data_node: Arc::new(DataNode {
            id: ID::new(),
            hash: Hash::new(),
            lineage: Lineage::new(),
            fields: DataNodeFields::new(),
            description: "Root node".to_string()
        }),
    });

    normalized.contexts.insert(read_lock!(root).id.clone(), root_context.clone());
    normalized.contexts_lookup.insert(read_lock!(root).id.clone(), root_context);

    Ok(normalized)
}
