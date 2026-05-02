use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};

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
use crate::transformation::{
    CanonicalizationTransformation,
    RelationshipTransformation,
    TraversalTransformation,
};
use crate::graph_node::Graph;
use crate::graph_node::GraphNode;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::normal_context::NormalContext;
use crate::data_node::DataNode;
use crate::network_relationship::NetworkRelationshipType;

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
    let stage = execution_context.enter_stage("Building normalized graph");

    let (contexts, normalized_graph_root) = build_normalized_graph(
        Arc::clone(&provider),
        Arc::clone(&meta_context),
        &options,
    )?;

    {
        let mut lock = write_lock!(meta_context);
        lock.update_normalized_graph(contexts, normalized_graph_root);
    }


    stage.finish();

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

fn build_normalized_graph<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options
) -> Result<
    (
        HashMap<ID, Arc<NormalContext>>,
        Arc<RwLock<GraphNode>>,
    ),
    Errors,
> {
    log::trace!("In build_normalized_graph");

    let normalized = Arc::new(RwLock::new(GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        description: String::from("placeholder description"),
        hash: Hash::new(),
        subgraph_hash: Hash::new(),
        lineage: Lineage::new(),
        children: Vec::new(),
    }));
    let mut contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();

    let basis_graph: BasisGraph = read_lock!(meta_context).basis_graph.clone().unwrap();
    let canonicalization: CanonicalizationTransformation = basis_graph.canonicalization;
    let graph_root = read_lock!(meta_context).graph_root.clone().unwrap();

    let mut queue = VecDeque::new();
    queue.push_back(graph_root);

    while let Some(current) = queue.pop_front() {
        let subgraph_hash = read_lock!(current).subgraph_hash.clone();

        let is_canonical = {
            if let Some(basis_network) = read_lock!(meta_context).get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)? {
                if let Some(basis_network) = canonicalization.transform(vec![basis_network])?.first() {
                    log::info!("Found a canonical network");

                    process_canonical_network(
                        Arc::clone(&meta_context),
                        Arc::clone(&normalized),
                        Arc::clone(&current),
                        &mut contexts,
                    )?;

                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if !is_canonical {
            for child in read_lock!(current).children.iter() {
                queue.push_back(Arc::clone(child));
            }
        }
    }

    Ok((contexts, normalized))
}

fn process_canonical_network(
    meta_context: Arc<RwLock<MetaContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    contexts: &mut HashMap<ID, Arc<NormalContext>>
) -> Result<(), Errors> {

    let basis_graph: BasisGraph = read_lock!(meta_context).basis_graph.clone().unwrap();
    let relationships: Vec<RelationshipTransformation> = basis_graph.relationships.unwrap();

    let network = process_network(
        Arc::clone(&meta_context),
        Arc::clone(&normalized_parent_node),
        Arc::clone(&current_node),
        contexts,
    )?;

    let subgraph_hash: Hash = read_lock!(current_node).subgraph_hash.clone();

    let basis_network: Arc<BasisNetwork> = {
        let lock = read_lock!(meta_context);
        lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?.unwrap()
    };

    let current_relationships: Vec<&RelationshipTransformation> = relationships
        .iter()
        .filter(|item| item.from == basis_network.id)
        .collect();

    for relationship in current_relationships.iter() {
        match relationship.relationship_type {
            NetworkRelationshipType::Composition => {
                process_composition_relationship(
                    Arc::clone(&meta_context),
                    Arc::clone(&normalized_parent_node),
                    Arc::clone(&current_node),
                    *relationship,
                    contexts,
                )?;
            }
            NetworkRelationshipType::ParentChild => {
                todo!();
            }
        }
    }

    Ok(())
}

fn process_composition_relationship(
    meta_context: Arc<RwLock<MetaContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    relationship: &RelationshipTransformation,
    contexts: &mut HashMap<ID, Arc<NormalContext>>
) -> Result<(), Errors> {
    log::trace!("In process_composition_relationship");

    let basis_graph: BasisGraph = read_lock!(meta_context).basis_graph.clone().unwrap();
    let traversals: Option<Vec<TraversalTransformation>> = basis_graph.traversals;

    let Some(traversals) = traversals else {
        panic!("Processing composition relationship, but no traversals available on basis graph");
    };

    let Some(traversal) = traversals.iter().find(|item| item.relationship_id == relationship.id) else {
        panic!("Processing composition relationship, but no traversal is available for this relationship");
    };

    if let Some(target_network) = traversal.transform(
        Arc::clone(&meta_context),
        Arc::clone(&current_node),
    )? {
        log::info!("Traversal found target network");

        process_network(
            Arc::clone(&meta_context),
            Arc::clone(&normalized_parent_node),
            Arc::clone(&target_network),
            contexts
        )?;
    } else {
        log::warn!("Traversal could not be applied to find target network");
    }

    Ok(())
}

fn process_network(
    meta_context: Arc<RwLock<MetaContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    contexts: &mut HashMap<ID, Arc<NormalContext>>
) -> Result<(), Errors> {

    fn recurse(
        meta_context: Arc<RwLock<MetaContext>>,
        current_node: Graph,
        parent_normalized_node: Graph,
        contexts: &mut HashMap<ID, Arc<NormalContext>>
    ) -> Result<(), Errors> {
        let mut current_normalized: Option<Graph> = None;

        let subgraph_hash: Hash = read_lock!(current_node).subgraph_hash.clone();

        let maybe_basis_network = {
            let lock = read_lock!(meta_context);
            lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?
        };

        if let Some(basis_network) = maybe_basis_network {
            if let Some(normalized_data_node) = process_node(
                Arc::clone(&meta_context),
                Arc::clone(&current_node)
            )? {
                let normalized_data_node = Arc::new(normalized_data_node);

                let normalized_graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
                    Arc::clone(&normalized_data_node),
                    vec![parent_normalized_node.clone()]
                )));

                current_normalized = Some(Arc::clone(&normalized_graph_node));

                write_lock!(parent_normalized_node).children.push(Arc::clone(&normalized_graph_node));

                let normal_context = Arc::new(NormalContext {
                    id: ID::new(),
                    network_name: basis_network.transformation.image.clone(),
                    network_description: basis_network.description.clone(),
                    graph_node: Arc::clone(&normalized_graph_node),
                    data_node: Arc::clone(&normalized_data_node),
                });

                contexts.insert(
                    normalized_data_node.id.clone(),
                    Arc::clone(&normal_context)
                );
                contexts.insert(
                    read_lock!(normalized_graph_node).id.clone(),
                    Arc::clone(&normal_context)
                );
            }
        }

        let parent_for_children = current_normalized.clone()
            .unwrap_or_else(|| parent_normalized_node.clone());

        let basis_graph: BasisGraph = read_lock!(meta_context).basis_graph.clone().unwrap();
        let canonicalization: CanonicalizationTransformation = basis_graph.canonicalization;

        for child in &read_lock!(current_node).children {
            let subgraph_hash: Hash = read_lock!(child).subgraph_hash.clone();

            let basis_network = {
                let lock = read_lock!(meta_context);
                lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?
            };

            if let Some(basis_network) = basis_network {
                if let Some(canonical) = canonicalization
                    .transform(vec![Arc::clone(&basis_network)])?
                    .first()
                {
                    log::info!("Found a canonical network");

                    let canonical_graph = process_canonical_network(
                        Arc::clone(&meta_context),
                        Arc::clone(&parent_for_children),
                        Arc::clone(&child),
                        contexts,
                    )?;
                    continue;
                }

            } else {
                recurse(
                    Arc::clone(&meta_context),
                    Arc::clone(&child),
                    parent_for_children.clone(),
                    contexts
                )?;
            }
        }

        Ok(())
    }

    recurse(
        Arc::clone(&meta_context),
        Arc::clone(&current_node),
        Arc::clone(&normalized_parent_node),
        contexts
    )?;

    Ok(())
}

fn process_node(
    meta_context: Arc<RwLock<MetaContext>>,
    node: Graph,
) -> Result<Option<DataNode>, Errors> {
    let context = {
        let lock = read_lock!(meta_context);
        let contexts = lock.contexts.clone().unwrap();

        contexts.get(&read_lock!(node).id).cloned().unwrap()
    };
    let data_node = &context.data_node;
    let basis_lineage = context.basis_lineage().clone();

    if let Some(basis_lineage) = basis_lineage {
        let basis_node = {
            let lock = read_lock!(meta_context);
            lock.get_basis_node_by_lineage(&basis_lineage)
                .expect("Could not get basis node by lineage")
                .unwrap()
        };

        let data_nodes: Vec<DataNode> = basis_node.transformations
            .clone()
            .into_iter()
            .map(|transformation| {
                transformation
                    .transform(Arc::clone(&data_node))
                    .expect("Could not transform data node")
            })
            .filter(|data_node| !data_node.fields.is_empty())
            .collect();

        if !data_nodes.is_empty() {
            let normalized_data_node = DataNode::from_data_nodes(data_nodes);

            return Ok(Some(normalized_data_node));
        }
    }

    Ok(None)
}
