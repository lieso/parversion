use futures::future::try_join_all;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::basis_graph::BasisGraph;
use crate::data_node::{DataNode, DataNodeFields};
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::field_analysis::generate_basis_fields;
use crate::graph_analysis::generate_basis_graph;
use crate::graph_node::GraphNode;
use crate::group_analysis::{generate_basis_groups, resolve_context_groups};
use crate::network_analysis::generate_basis_networks;
use crate::node_analysis::generate_basis_nodes;
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::normalization_context::NormalizationContext;
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::reports::{
    report_basis_fields, report_basis_graph, report_basis_groups, report_basis_networks,
    report_basis_nodes,
};

pub async fn normalize<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    document: Document,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Vec<Arc<RwLock<NormalizationContext>>>, Errors> {
    log::trace!("In normalize");

    let start = Instant::now();
    let stage = execution_context.enter_stage("Initialization");

    let normalization_contexts = init_normalization_context(document, options).await?;

    stage.finish();
    let elapsed = start.elapsed();
    log::info!("init_normalization_context: {:.2?}", elapsed);

    let normalizations = try_join_all(normalization_contexts.iter().map(|normalization_context| {
        run_pipeline(
            Arc::clone(&normalization_context),
            Arc::clone(&provider),
            Arc::clone(&reasoner),
            options,
            execution_context.clone(),
        )
    }))
    .await?;

    Ok(normalizations)
}

async fn normalize_html(
    document: Document,
    _options: &Options,
) -> Result<Vec<Arc<RwLock<NormalizationContext>>>, Errors> {
    let (meta_contexts, _other_documents) = document.to_meta_context()?;

    let normalization_contexts = meta_contexts
        .into_iter()
        .map(|meta_context| {
            let normalization_context = Arc::new(RwLock::new(NormalizationContext::new()));

            {
                let mut lock = write_lock!(normalization_context);
                lock.update_meta_context(meta_context);
            }

            normalization_context
        })
        .collect();

    Ok(normalization_contexts)
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

    let normalization_contexts = normalize(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        document,
        _options,
        execution_context,
    )
    .await?;

    let normalized_documents: Vec<Document> = normalization_contexts
        .iter()
        .map(|normalization_context| {
            Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)
        })
        .collect::<Result<_, _>>()?;

    Ok(Package {
        documents: normalized_documents,
    })
}

pub async fn normalize_text<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    text: String,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Vec<Arc<RwLock<NormalizationContext>>>, Errors> {
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
) -> Result<Vec<Document>, Errors> {
    log::trace!("In normalize_text_to_document");

    let normalization_contexts = normalize_text(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        text,
        _options,
        metadata,
        execution_context,
    )
    .await?;

    normalization_contexts
        .iter()
        .map(|normalization_context| {
            Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)
        })
        .collect::<Result<_, _>>()
}

pub async fn normalize_file<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    path: &str,
    _options: &Options,
    metadata: &Metadata,
    execution_context: Arc<ExecutionContext>,
) -> Result<Vec<Arc<RwLock<NormalizationContext>>>, Errors> {
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
) -> Result<Vec<Document>, Errors> {
    log::trace!("In normalize_file_to_document");
    log::debug!("file path: {}", path);

    let normalization_contexts = normalize_file(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        path,
        _options,
        metadata,
        execution_context,
    )
    .await?;

    normalization_contexts
        .iter()
        .map(|normalization_context| {
            Document::from_normalized_graph(Arc::clone(&normalization_context), document_format)
        })
        .collect::<Result<_, _>>()
}

async fn init_normalization_context(
    document: Document,
    options: &Options,
) -> Result<Vec<Arc<RwLock<NormalizationContext>>>, Errors> {
    log::trace!("In init_normalization_context");

    match document.document_type {
        DocumentType::Html => normalize_html(document, options).await,
        DocumentType::Json => {
            unimplemented!()
        }
        DocumentType::PlainText => {
            unimplemented!()
        }
        DocumentType::JavaScript => {
            unimplemented!()
        }
        DocumentType::Xml => {
            unimplemented!()
        }
    }
}

fn build_normalized_graph(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    _options: &Options,
) -> Result<NormalMetaContext, Errors> {
    log::trace!("In build_normalized_graph");

    let basis_graph: Arc<BasisGraph> = {
        let lock = read_lock!(normalization_context);
        lock.basis_graph
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(
                    "Basis graph not provided in normalization context".to_string(),
                )
            })?
            .clone()
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

    let mut normalized =
        basis_graph.apply(Arc::clone(&normalization_context), Arc::clone(&root))?;

    let root_context = Arc::new(NormalContext {
        id: ID::new(),
        network_name: None,
        network_description: None,
        graph_node: Arc::clone(&root),
        data_node: Arc::new(DataNode {
            id: ID::new(),
            hash: Hash::new(),
            lineage: Lineage::new(),
            fields: DataNodeFields::new(),
            description: "Root node".to_string(),
        }),
        contexts: Vec::new(),
    });

    normalized
        .contexts
        .insert(read_lock!(root).id.clone(), root_context.clone());
    normalized
        .contexts_lookup
        .insert(read_lock!(root).id.clone(), root_context);

    Ok(normalized)
}

async fn run_pipeline<P: Provider, R: Reasoner>(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    provider: Arc<P>,
    reasoner: Arc<R>,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<Arc<RwLock<NormalizationContext>>, Errors> {
    let process = execution_context.enter_stage("Normalization");

    let start = Instant::now();
    let stage = process.enter_stage("Field analysis");

    let basis_fields = generate_basis_fields(
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
        report_basis_fields(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = process.enter_stage("Group analysis");

    let basis_groups = generate_basis_groups(
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

    let (context_groups, context_to_group) =
        resolve_context_groups(Arc::clone(&normalization_context))?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_context_groups(context_groups, context_to_group);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_groups: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_groups(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = process.enter_stage("Node analysis");

    log::info!("Getting basis nodes");
    let (basis_nodes, basis_node_contexts, context_basis_node) = generate_basis_nodes(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        normalization_context.clone(),
        &options,
        &stage,
    )
    .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_nodes(basis_nodes, basis_node_contexts, context_basis_node);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_nodes: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_nodes(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = process.enter_stage("Network analysis");

    log::info!("Generating basis networks");
    let (basis_networks,) = generate_basis_networks(
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
    log::info!("generate_basis_networks: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_networks(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = process.enter_stage("Graph analysis");

    log::info!("Generating basis graph");
    let basis_graph = generate_basis_graph(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        Arc::clone(&normalization_context),
        &options,
        &stage,
    )
    .await?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_basis_graph(basis_graph);
    }

    let elapsed = start.elapsed();
    log::info!("generate_basis_graph: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_basis_graph(Arc::clone(&normalization_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = process.enter_stage("Building normalized graph");

    let normalized = build_normalized_graph(Arc::clone(&normalization_context), &options)?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_normalized_graph(normalized);
    }

    let elapsed = start.elapsed();
    log::info!("build_normalized_graph: {:.2?}", elapsed);

    stage.finish();
    process.finish();

    Ok(normalization_context)
}
