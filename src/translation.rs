use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::collections::HashMap;

use crate::document::{Document, DocumentType, DocumentRole};
use crate::document_format::DocumentFormat;
use crate::node_analysis::generate_translation_nodes;
use crate::normalization;
use crate::package::Package;
use crate::prelude::*;
use crate::provider::Provider;
use crate::translation_context::TranslationContext;
use crate::reports::report_translation_nodes;
use crate::normal_meta_context::NormalMetaContext;
use crate::graph_node::{Graph, GraphNode};
use crate::data_node::{DataNode, DataNodeFields};
use crate::normal_context::NormalContext;
use crate::translation_node::TranslationNode;

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

    let translation_context = init_translation_context(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        source,
        target,
        options,
        execution_context.clone(),
    )
    .await?;

    let elapsed = start.elapsed();
    log::info!("init_translation_context: {:.2?}", elapsed);

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Node translation");

    let (translation_nodes, translation_nodes_lookup, target_node_lookup) = generate_translation_nodes(
        Arc::clone(&provider),
        Arc::clone(&reasoner),
        Arc::clone(&translation_context),
        options,
        &stage,
    )
    .await?;

    {
        let mut lock = write_lock!(translation_context);
        lock.update_translation_nodes(translation_nodes, translation_nodes_lookup, target_node_lookup);
    }

    let elapsed = start.elapsed();
    log::info!("generate_translation_nodes: {:.2?}", elapsed);

    #[cfg(debug_assertions)]
    {
        report_translation_nodes(Arc::clone(&translation_context)).await?;
    }

    stage.finish();

    let start = Instant::now();
    let stage = execution_context.enter_stage("Building translated graph");

    let translated = build_translated_graph(Arc::clone(&translation_context), &options)?;

    {
        let mut lock = write_lock!(translation_context);
        lock.update_translated_graph(translated.clone());
    }

    let elapsed = start.elapsed();
    log::info!("build_translated_graph: {:.2?}", elapsed);

    stage.finish();

    Ok(translation_context)
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

fn build_translated_graph(
    translation_context: Arc<RwLock<TranslationContext>>,
    _options: &Options,
) -> Result<NormalMetaContext, Errors> {
    log::trace!("In build_translated_graph");

    let normalization_contexts = {
        let lock = read_lock!(translation_context);
        lock.source.clone()
    };

    let root = Arc::new(RwLock::new(GraphNode::detached()));
    let mut contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
    let mut contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();

    // TODO: what if different normalization contexts map to the same network instance on the target
    // document?
    for normalization_context in normalization_contexts {
        let source_root = {
            let lock = read_lock!(normalization_context);
            let normalized = lock.normalized
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError(
                        "Normalized graph not provided in normalization context".to_string()
                    )
                })?;

            normalized.graph_root.clone()
        };

        recurse_source_graph(
            Arc::clone(&root),
            source_root,
            Arc::clone(&translation_context),
            Arc::clone(&normalization_context),
            &mut HashMap::new(),
            &mut contexts,
            &mut contexts_lookup,
        )?;
    }

    let normal_meta_context = NormalMetaContext::new(
        contexts,
        Arc::clone(&root),
        contexts_lookup,
    );

    Ok(normal_meta_context)
}

fn recurse_source_graph(
    target: Graph,
    source: Graph,
    translation_context: Arc<RwLock<TranslationContext>>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    instance_counter: &mut HashMap<TranslationNodeID, usize>,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
) -> Result<(), Errors> {
    if let Some(translation_node) = {
        let acyclic_lineage = &read_lock!(source).acyclic_lineage;
        let lock = read_lock!(translation_context);
        let lookup = lock.translation_nodes_lookup
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientTranslationContextError(
                    "Translation node lookup not provided in translation context".to_string()
                )
            })?;

        lookup.get(acyclic_lineage).cloned()
    } {
        log::info!("Found a translation node");

        let instance = instance_counter
            .entry(translation_node.id.clone())
            .or_insert(0);

        let canonical_target_context = get_canonical_target_context(
            Arc::clone(&translation_context),
            Arc::clone(&translation_node),
        )?;

        let leaf = build_ancestral_chain(
            Arc::clone(&target),
            Arc::clone(&translation_context),
            Arc::clone(&translation_node),
            contexts,
            contexts_lookup,
            &instance,
            Arc::clone(&canonical_target_context),
        )?;

        build_translated_node(
            Arc::clone(&normalization_context),
            Arc::clone(&translation_node),
            Arc::clone(&source),
            Arc::clone(&leaf),
            contexts,
            contexts_lookup,
            Arc::clone(&canonical_target_context),
        )?;

        *instance += 1;
    }

    for source_child in &read_lock!(source).children {
        recurse_source_graph(
            Arc::clone(&target),
            Arc::clone(&source_child),
            Arc::clone(&translation_context),
            Arc::clone(&normalization_context),
            instance_counter,
            contexts,
            contexts_lookup
        )?;
    }

    Ok(())
}

fn build_translated_node(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    translation_node: Arc<TranslationNode>,
    source_node: Graph,
    leaf: Graph,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
    canonical_target_context: Arc<NormalContext>
) -> Result<(), Errors> {
    log::trace!("In build_translated_node");

    let translated_data_node = {
        let source_context = {
            let lock = read_lock!(normalization_context);
            let normalized = lock.normalized
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError(
                        "Normalized graph not provided in normalization context".to_string()
                    )
                })?;

            normalized
                .contexts_lookup
                .get(&read_lock!(source_node).id)
                .cloned()
        };

        if let Some(source_context) = source_context {
            let source_data_node = &source_context.data_node;

            let transformed = translation_node
                .transformations
                .iter()
                .map(|transformation| transformation.transform(source_data_node.clone()))
                .collect::<Result<Vec<DataNode>, Errors>>()?;

            DataNode::from_data_nodes(transformed)
        } else {
            return Err(Errors::UnexpectedError("Found a translation node, but did not find a data node to translate".to_string()));
        }
    };

    let normal_context = Arc::new(NormalContext {
        id: ID::new(),
        network_name: canonical_target_context.network_name.clone(),
        network_description: canonical_target_context.network_description.clone(),
        graph_node: Arc::clone(&leaf),
        data_node: Arc::new(translated_data_node),
        contexts: Vec::new(),
    });

    contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
    contexts_lookup.insert(read_lock!(leaf).id.clone(), Arc::clone(&normal_context));

    Ok(())
}

fn build_ancestral_chain(
    root: Graph,
    translation_context: Arc<RwLock<TranslationContext>>,
    translation_node: Arc<TranslationNode>,
    contexts: &mut HashMap<ID, Arc<NormalContext>>,
    contexts_lookup: &mut HashMap<ID, Arc<NormalContext>>,
    instance: &usize,
    canonical_target_context: Arc<NormalContext>
) -> Result<Graph, Errors> {
    log::trace!("In build_ancestral_chain");

    let ancestors = {
        let mut ancestors: Vec<Graph> = Vec::new();
        let mut current_parents = read_lock!(canonical_target_context.graph_node).parents.clone();

        while !current_parents.is_empty() {
            let parent = current_parents[0].clone();
            ancestors.push(parent.clone());
            current_parents = read_lock!(parent).parents.clone();
        }

        ancestors.reverse();
        ancestors
    };



    let mut current_result_node = Arc::clone(&root);

    for ancestor in ancestors {

        let current_children = {
            let lock = read_lock!(current_result_node);
            lock.children.clone()
        };


        let matching_children: Vec<Graph> = current_children
            .iter()
            .filter(|item| {
                read_lock!(item).acyclic_lineage == read_lock!(ancestor).acyclic_lineage
            })
            .cloned()
            .collect();


        if let Some(existing_ancestor) = matching_children.get(*instance) {

            current_result_node = Arc::clone(&existing_ancestor);

        } else {

            let child = Arc::new(RwLock::new(read_lock!(ancestor).clone_without_relations()));






            let ancestor_id = read_lock!(ancestor).id.clone();

            let original: Option<Arc<NormalContext>> = {
                let lock = read_lock!(translation_context);
                let mut found = None;

                for target in &lock.target {
                    let target_lock = read_lock!(target);

                    if let Some(normalized) = target_lock.normalized.as_ref() {
                        if let Some(context) = normalized.contexts_lookup.get(&ancestor_id) {
                            found = Some(Arc::clone(context));
                            break;
                        }
                    }
                }

                found
            };


            if let Some(original) = original {
                let normal_context = Arc::new(NormalContext {
                    id: ID::new(),
                    network_name: original.network_name.clone(),
                    network_description: original.network_description.clone(),
                    graph_node: Arc::clone(&child),
                    data_node: Arc::new(DataNode {
                        id: ID::new(),
                        fields: DataNodeFields::new(),
                        description: "placeholder".to_string(),
                    }),
                    contexts: Vec::new(),
                });

                contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
                contexts_lookup.insert(read_lock!(child).id.clone(), Arc::clone(&normal_context));
            }





            {
                let mut parent_lock = write_lock!(current_result_node);

                parent_lock.children.push(Arc::clone(&child));

            }

            {
                let mut child_lock = write_lock!(child);

                child_lock.parents.push(Arc::clone(&current_result_node));
            }

            current_result_node = Arc::clone(&child);

        }
    }

    Ok(Arc::clone(&current_result_node))
}

fn get_canonical_target_context(
    translation_context: Arc<RwLock<TranslationContext>>,
    translation_node: Arc<TranslationNode>,
) -> Result<Arc<NormalContext>, Errors> {
    log::trace!("In get_canonical_target_context");

    let Some(target_contexts) = ({
        let lock = read_lock!(translation_context);
        let lookup = lock
            .target_node_lookup
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientTranslationContextError(
                    "Target node lookup not provided in translation context".to_string()
                )
            })?;

        lookup.get(&translation_node.right_lineage).cloned()
    }) else {
        return Err(Errors::UnexpectedError("Found a translation node, but could not find target document node".to_string()));
    };

    if target_contexts.is_empty() {
        return Err(Errors::UnexpectedError("Found zero target contexts".to_string()));
    }

    let canonical_target_context = target_contexts
        .first()
        .unwrap();

    Ok(Arc::clone(&canonical_target_context))
}
