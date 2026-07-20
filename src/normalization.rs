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
    get_network_relationships,
    generate_basis_networks
};
use crate::reports::{
    report_basis_groups,
    report_basis_fields,
    report_basis_nodes
};
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
use crate::basis_group::BasisGroup;
use crate::normal_context::NormalContext;
use crate::data_node::DataNode;
use crate::network_relationship::NetworkRelationshipType;
use crate::classification::Classification;

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
    let basis_nodes =
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
        lock.update_basis_nodes(basis_nodes);
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
    let basis_networks =
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

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Network relationships");

    log::info!("Generating network relationships");

    let basis_graph =
        get_network_relationships(
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
    log::info!("get_network_relationships: {:.2?}", elapsed);

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Building normalized graph");

    let (contexts, normalized_graph_root) = build_normalized_graph(
        Arc::clone(&provider),
        Arc::clone(&normalization_context),
        &options,
    )?;

    {
        let mut lock = write_lock!(normalization_context);
        lock.update_normalized_graph(contexts, normalized_graph_root);
    }

    let elapsed = start.elapsed();
    log::info!("buld_normalized_graph: {:.2?}", elapsed);

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
) -> Result<
    (
        HashMap<ID, Arc<NormalContext>>,
        Arc<RwLock<GraphNode>>,
    ),
    Errors,
> {
    log::trace!("In build_normalized_graph");

    unimplemented!()

    //let classification: Arc<Classification> = {
    //    let lock = read_lock!(normalization_context);
    //    lock.classification.clone().ok_or(Errors::ClassificationNotFound)?
    //};
    //let normalized = Arc::new(RwLock::new(GraphNode {
    //    id: ID::new(),
    //    parents: Vec::new(),
    //    description: String::from("placeholder description"),
    //    hash: Hash::new(),
    //    subgraph_hash: Hash::new(),
    //    lineage: Lineage::new(),
    //    children: Vec::new(),
    //}));

    //let mut contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
    //let mut visited: HashSet<ID> = HashSet::new();

    //let data_node = Arc::new(DataNode {
    //    id: ID::new(),
    //    hash: Hash::new(),
    //    lineage: Lineage::new(),
    //    fields: HashMap::new(),
    //    description: "placeholder".to_string()
    //});

    //let root_context = Arc::new(NormalContext {
    //    id: ID::new(),
    //    network_name: Some(classification.name.clone()),
    //    network_description: Some(classification.description.clone()),
    //    graph_node: Arc::clone(&normalized),
    //    data_node: Arc::clone(&data_node),
    //});
    //contexts.insert(
    //    data_node.id.clone(),
    //    Arc::clone(&root_context)
    //);
    //contexts.insert(
    //    read_lock!(normalized).id.clone(),
    //    Arc::clone(&root_context)
    //);

    //let basis_graph: BasisGraph = read_lock!(normalization_context).basis_graph.clone().unwrap();
    //let canonicalization: CanonicalizationTransformation = basis_graph.canonicalization;
    //let graph_root = read_lock!(normalization_context).meta_context.as_ref().unwrap().graph_root.clone();

    //let mut queue = VecDeque::new();
    //queue.push_back(graph_root);

    //while let Some(current) = queue.pop_front() {
    //    let subgraph_hash = read_lock!(current).subgraph_hash.clone();

    //    let is_canonical = {
    //        if let Some(basis_network) = read_lock!(normalization_context).get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)? {
    //            if let Some(basis_network) = canonicalization.transform(vec![basis_network])?.first() {
    //                process_canonical_network(
    //                    Arc::clone(&normalization_context),
    //                    Arc::clone(&normalized),
    //                    Arc::clone(&current),
    //                    &mut contexts,
    //                    &mut visited,
    //                )?;

    //                true
    //            } else {
    //                false
    //            }
    //        } else {
    //            false
    //        }
    //    };

    //    if !is_canonical {
    //        for child in read_lock!(current).children.iter() {
    //            queue.push_back(Arc::clone(child));
    //        }
    //    }
    //}

    //Ok((contexts, normalized))
}

fn process_canonical_network(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    visited: &mut HashSet<ID>,
) -> Result<(), Errors> {
    unimplemented!()
    //let basis_graph: BasisGraph = read_lock!(normalization_context).basis_graph.clone().unwrap();
    //let relationships: Vec<RelationshipTransformation> = basis_graph.relationships.unwrap();
    //let subgraph_hash: Hash = read_lock!(current_node).subgraph_hash.clone();
    //let basis_network: Arc<BasisNetwork> = {
    //    let lock = read_lock!(normalization_context);
    //    lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?.unwrap()
    //};

    //let current_relationships: Vec<&RelationshipTransformation> = relationships
    //    .iter()
    //    .filter(|item| item.from == basis_network.id || item.to == basis_network.id)
    //    .collect();

    //// No relationships, but still a canonical network so we include it
    //if current_relationships.is_empty() {
    //    process_network(
    //        Arc::clone(&normalization_context),
    //        Arc::clone(&normalized_parent_node),
    //        Arc::clone(&current_node),
    //        contexts,
    //        visited,
    //    )?;
    //}

    //for relationship in current_relationships.iter() {
    //    match relationship.relationship_type {
    //        NetworkRelationshipType::Composition => {
    //            if relationship.from == basis_network.id {
    //                let normalized_data_node = Arc::new(DataNode {
    //                    id: ID::new(),
    //                    hash: Hash::new(),
    //                    lineage: Lineage::new(),
    //                    fields: HashMap::new(),
    //                    description: "Merged network".to_string()
    //                });
    //                let normalized_graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
    //                    Arc::clone(&normalized_data_node),
    //                    vec![normalized_parent_node.clone()]
    //                )));

    //                write_lock!(normalized_parent_node).children.push(Arc::clone(&normalized_graph_node));

    //                let normal_context = Arc::new(NormalContext {
    //                    id: ID::new(),
    //                    network_name: Some(basis_network.transformation.image.clone()),
    //                    network_description: Some(basis_network.description.clone()),
    //                    graph_node: Arc::clone(&normalized_graph_node),
    //                    data_node: Arc::clone(&normalized_data_node),
    //                });

    //                contexts.insert(
    //                    normalized_data_node.id.clone(),
    //                    Arc::clone(&normal_context)
    //                );
    //                contexts.insert(
    //                    read_lock!(normalized_graph_node).id.clone(),
    //                    Arc::clone(&normal_context)
    //                );

    //                process_network(
    //                    Arc::clone(&normalization_context),
    //                    Arc::clone(&normalized_graph_node),
    //                    Arc::clone(&current_node),
    //                    contexts,
    //                    visited,
    //                )?;

    //                process_composition_relationship(
    //                    Arc::clone(&normalization_context),
    //                    Arc::clone(&normalized_graph_node),
    //                    Arc::clone(&current_node),
    //                    *relationship,
    //                    contexts,
    //                    visited,
    //                )?;
    //            }
    //        }
    //        NetworkRelationshipType::ParentChild => {
    //            todo!();
    //        }
    //    }
    //}

    //Ok(())
}

fn process_composition_relationship(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    relationship: &RelationshipTransformation,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    visited: &mut HashSet<ID>,
) -> Result<(), Errors> {
    let basis_graph: BasisGraph = read_lock!(normalization_context).basis_graph.clone().unwrap();
    let traversals: Option<Vec<TraversalTransformation>> = basis_graph.traversals;

    let Some(traversals) = traversals else {
        panic!("Processing composition relationship, but no traversals available on basis graph");
    };

    let Some(traversal) = traversals.iter().find(|item| item.relationship_id == relationship.id) else {
        panic!("Processing composition relationship, but no traversal is available for this relationship");
    };

    if let Some(target_network) = traversal.transform(
        Arc::clone(&normalization_context),
        Arc::clone(&current_node),
    )? {
        let target_id = read_lock!(target_network).id.clone();
        if visited.contains(&target_id) {
            log::info!("Composition target is already being processed — skipping to prevent cycle");
        } else {
            process_network(
                Arc::clone(&normalization_context),
                Arc::clone(&normalized_parent_node),
                Arc::clone(&target_network),
                contexts,
                visited,
            )?;
        }
    } else {
        log::warn!("Traversal could not be applied to find target network");
    }

    Ok(())
}

fn process_network(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    normalized_parent_node: Graph,
    current_node: Graph,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    visited: &mut HashSet<ID>,
) -> Result<(), Errors> {
    unimplemented!()
    //visited.insert(read_lock!(current_node).id.clone());

    //fn recurse(
    //    normalization_context: Arc<RwLock<NormalizationContext>>,
    //    current_node: Graph,
    //    parent_normalized_node: Graph,
    //    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    //    visited: &mut HashSet<ID>,
    //) -> Result<(), Errors> {
    //    let basis_graph: BasisGraph = read_lock!(normalization_context).basis_graph.clone().unwrap();
    //    let canonicalization: CanonicalizationTransformation = basis_graph.canonicalization;
    //    
    //    let normalized_data_node = process_node(
    //        Arc::clone(&normalization_context),
    //        Arc::clone(&current_node)
    //    )?;

    //    let normalized_data_node = Arc::new(normalized_data_node);

    //    let normalized_graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
    //        Arc::clone(&normalized_data_node),
    //        vec![parent_normalized_node.clone()]
    //    )));

    //    write_lock!(parent_normalized_node).children.push(Arc::clone(&normalized_graph_node));

    //    let normal_context = {
    //        let subgraph_hash: Hash = read_lock!(current_node).subgraph_hash.clone();

    //        let basis_network = {
    //            let lock = read_lock!(normalization_context);
    //            lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?
    //        };

    //        if let Some(basis_network) = basis_network {
    //            Arc::new(NormalContext {
    //                id: ID::new(),
    //                network_name: Some(basis_network.transformation.image.clone()),
    //                network_description: Some(basis_network.description.clone()),
    //                graph_node: Arc::clone(&normalized_graph_node),
    //                data_node: Arc::clone(&normalized_data_node),
    //            })
    //        } else {
    //            Arc::new(NormalContext {
    //                id: ID::new(),
    //                network_name: None,
    //                network_description: None,
    //                graph_node: Arc::clone(&normalized_graph_node),
    //                data_node: Arc::clone(&normalized_data_node),
    //            })
    //        }
    //    };

    //    contexts.insert(
    //        normalized_data_node.id.clone(),
    //        Arc::clone(&normal_context)
    //    );
    //    contexts.insert(
    //        read_lock!(normalized_graph_node).id.clone(),
    //        Arc::clone(&normal_context)
    //    );

    //    for child in &read_lock!(current_node).children {
    //        let subgraph_hash: Hash = read_lock!(child).subgraph_hash.clone();

    //        let basis_network = {
    //            let lock = read_lock!(normalization_context);
    //            lock.get_basis_network_by_lineage_and_subgraph_hash(&subgraph_hash)?
    //        };

    //       if let Some(basis_network) = basis_network {
    //           if let Some(canonical) = canonicalization
    //               .transform(vec![Arc::clone(&basis_network)])?
    //               .first()
    //           {
    //               process_canonical_network(
    //                   Arc::clone(&normalization_context),
    //                   Arc::clone(&normalized_graph_node),
    //                   Arc::clone(&child),
    //                   contexts,
    //                   visited,
    //               )?;
    //               continue;
    //           }
    //       }

    //       recurse(
    //           Arc::clone(&normalization_context),
    //           Arc::clone(&child),
    //           Arc::clone(&normalized_graph_node),
    //           contexts,
    //           visited,
    //       )?;
    //    }

    //    Ok(())
    //}

    //recurse(
    //    Arc::clone(&normalization_context),
    //    Arc::clone(&current_node),
    //    Arc::clone(&normalized_parent_node),
    //    contexts,
    //    visited,
    //)?;

    //Ok(())
}

fn process_node(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    node: Graph,
) -> Result<DataNode, Errors> {
    let context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.as_ref().unwrap().contexts_lookup
            .get(&read_lock!(node).id)
            .cloned()
            .unwrap()
    };
    let context_to_group = {
        let lock = read_lock!(normalization_context);
        lock.context_to_group.clone().unwrap()
    };
    let data_node = &context.data_node;
    let maybe_basis_group: Option<Arc<BasisGroup>> = context_to_group.get(&context.id).cloned();

    let basis_lineage: Option<Lineage> = {
        if let Some(basis_group) = maybe_basis_group {
            Some(basis_group.get_basis_lineage())
        } else {
            None
        }
    };

    if let Some(basis_lineage) = basis_lineage {
        let basis_node = {
            let lock = read_lock!(normalization_context);
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
            .collect();

        let normalized_data_node = DataNode::from_data_nodes(data_nodes);

        Ok(normalized_data_node)
    } else {
        Ok(DataNode {
            id: ID::new(),
            hash: Hash::new(),
            lineage: Lineage::new(),
            fields: HashMap::new(),
            description: "placeholder".to_string()
        })
    }
}
