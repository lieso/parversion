use futures::future::try_join_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tokio::task;

use crate::basis_node::BasisNode;
use crate::config::CONFIG;
use crate::graph_node::Graph;
use crate::llm::{LLM, NodeGroupClassification};
use crate::meta_context::MetaContext;
use crate::path::Path;
use crate::prelude::*;
use crate::provider::Provider;
use crate::schema_context::SchemaContext;
use crate::transformation::{FieldTransformation, SchemaTransformation};
use crate::traversal::{get_original_document_condensed};
use crate::context::Context;

pub async fn get_translation_schema_transformations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<HashMap<Lineage, Arc<SchemaTransformation>>, Errors> {
    log::trace!("In get_translation_schema_transformations");
    let _ = execution_context;

    let schema_contexts: Vec<Arc<SchemaContext>> = {
        let lock = read_lock!(meta_context);

        lock.schema_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Schema contexts not provided in meta context".to_string(),
                )
            })?
            .values()
            .cloned()
            .collect()
    };
    let target_schema: String = {
        let lock = read_lock!(meta_context);

        let graph_root = lock
            .translation_schema_graph_root
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Translation schema graph root not provided in meta context".to_string(),
                )
            })?
            .clone();

        let schema_contexts_map: HashMap<ID, Arc<SchemaContext>> = {
            let lock = read_lock!(meta_context);
            lock.translation_schema_contexts.clone().unwrap()
        };

        let snippet = &SchemaContext::traverse_for_snippet(
            &schema_contexts_map,
            Arc::clone(&graph_root),
            &|_id| true,
            &|_id| false,
        );

        format!("{{ {} }}", snippet)
    };
    let target_schema = Arc::new(target_schema);

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

    if max_concurrency == 1 {
        let mut results = HashMap::new();

        for schema_context in schema_contexts {
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let result = get_translation_schema_transformation(
                cloned_provider,
                cloned_meta_context,
                schema_context.clone(),
                target_schema.clone(),
                options,
            )
            .await?;

            results.insert(result.lineage.clone(), Arc::new(result));
        }

        Ok(results)
    } else {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for schema_context in schema_contexts {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let cloned_target_schema = Arc::clone(&target_schema);
            let cloned_options = options.clone();

            let handle = task::spawn(async move {
                let _permit = permit;
                let transformation = get_translation_schema_transformation(
                    cloned_provider,
                    cloned_meta_context,
                    schema_context.clone(),
                    cloned_target_schema,
                    &cloned_options,
                )
                .await?;

                Ok((transformation.lineage.clone(), Arc::new(transformation)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(Lineage, Arc<SchemaTransformation>), Errors>> =
            try_join_all(handles).await?;

        let hashmap_results: HashMap<Lineage, Arc<SchemaTransformation>> =
            results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

pub async fn get_normal_schema_transformations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    execution_context: Arc<ExecutionContext>,
) -> Result<HashMap<Lineage, Arc<SchemaTransformation>>, Errors> {
    log::trace!("In get_normal_schema_transformations");
    let _ = execution_context;

    let schema_contexts: Vec<Arc<SchemaContext>> = {
        let lock = read_lock!(meta_context);

        lock.schema_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Normal schema contexts not provided in meta context".to_string(),
                )
            })?
            .values()
            .cloned()
            .collect()
    };

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

    if max_concurrency == 1 {
        let mut results = HashMap::new();

        for schema_context in schema_contexts {
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let result = get_normal_schema_transformation(
                cloned_provider,
                cloned_meta_context,
                schema_context.clone(),
                options,
            )
            .await?;

            results.insert(result.lineage.clone(), Arc::new(result));
        }

        Ok(results)
    } else {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for schema_context in schema_contexts {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let cloned_options = options.clone();

            let handle = task::spawn(async move {
                let _permit = permit;
                let transformation = get_normal_schema_transformation(
                    cloned_provider,
                    cloned_meta_context,
                    schema_context.clone(),
                    &cloned_options,
                )
                .await?;

                Ok((transformation.lineage.clone(), Arc::new(transformation)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(Lineage, Arc<SchemaTransformation>), Errors>> =
            try_join_all(handles).await?;

        let hashmap_results: HashMap<Lineage, Arc<SchemaTransformation>> =
            results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

pub async fn get_basis_nodes<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In get_basis_nodes");




    let context_groups = get_context_groups(Arc::clone(&provider), Arc::clone(&meta_context), options).await?;

    log::info!("Number of context groups: {}", context_groups.len());





    let document_summary = Arc::new(get_original_document_condensed(Arc::clone(&meta_context))?);

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for group in context_groups {
        stage_context.record_events("Node analysis", 0);

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let cloned_provider = Arc::clone(&provider);
        let cloned_meta_context = Arc::clone(&meta_context);
        let cloned_options = options.clone();
        let cloned_stage_context = stage_context.clone();
        let cloned_document_summary = Arc::clone(&document_summary);

        let handle = task::spawn(async move {
            let _permit = permit;
            let basis_node = get_basis_node(
                cloned_provider,
                cloned_meta_context,
                group,
                &cloned_options,
                &cloned_stage_context,
                &cloned_document_summary,
            )
            .await?;

            Ok((basis_node.id.clone(), Arc::new(basis_node)))
        });
        handles.push(handle);
    }

    let results: Vec<Result<(ID, Arc<BasisNode>), Errors>> = try_join_all(handles).await?;

    let hashmap_results: HashMap<ID, Arc<BasisNode>> =
        results.into_iter().collect::<Result<_, _>>()?;

    Ok(hashmap_results)
}

async fn get_normal_schema_transformation<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    schema_context: Arc<SchemaContext>,
    options: &Options,
) -> Result<SchemaTransformation, Errors> {
    log::trace!("In get_normal_schema_transformation");

    let lineage = &schema_context.lineage;

    if !options.regenerate {
        if let Some(schema_transformation) =
            provider.get_schema_transformation(&lineage, None).await?
        {
            log::info!("Provider has supplied normal schema transformation");

            return Ok(schema_transformation);
        };
    }

    let snippet = schema_context.generate_snippet(Arc::clone(&meta_context));

    unimplemented!()
}

async fn get_translation_schema_transformation<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    schema_context: Arc<SchemaContext>,
    target_schema: Arc<String>,
    options: &Options,
) -> Result<SchemaTransformation, Errors> {
    log::trace!("In get_translation_schema_transformation");

    let lineage = &schema_context.lineage;
    let schema_root: Graph = {
        let lock = read_lock!(meta_context);
        lock.translation_schema_graph_root.clone().ok_or_else(|| {
            Errors::DeficientMetaContextError(
                "Schema contexts not provided in meta context".to_string(),
            )
        })?
    };
    let subgraph_hash: Hash = {
        let lock = read_lock!(schema_root);
        lock.subgraph_hash.clone()
    };

    if !options.regenerate {
        if let Some(schema_transformation) = provider
            .get_schema_transformation(&lineage, Some(&subgraph_hash))
            .await?
        {
            log::info!("Provider has supplied translation schema transformtion");

            return Ok(schema_transformation);
        };
    }

    let result = LLM::translate_schema_node(
        Arc::clone(&meta_context),
        (*schema_context).clone(),
        Arc::clone(&target_schema),
    )
    .await?;

    if let Some((source, target)) = result {
        let schema_transformation = SchemaTransformation {
            id: ID::new(),
            timestamp: Timestamp::now(),
            description: schema_context.schema_node.description.clone(),
            key: schema_context.schema_node.name.clone(),
            source: Some(source),
            target: Some(target),
            lineage: lineage.clone(),
            subgraph_hash: Some(subgraph_hash.clone()),
        };

        provider
            .save_schema_transformation(
                &lineage,
                Some(&subgraph_hash),
                schema_transformation.clone(),
            )
            .await?;

        Ok(schema_transformation)
    } else {
        let schema_transformation = SchemaTransformation {
            id: ID::new(),
            timestamp: Timestamp::now(),
            description: schema_context.schema_node.description.clone(),
            key: schema_context.schema_node.name.clone(),
            source: None,
            target: None,
            lineage: lineage.clone(),
            subgraph_hash: Some(subgraph_hash.clone()),
        };

        provider
            .save_schema_transformation(
                &lineage,
                Some(&subgraph_hash),
                schema_transformation.clone(),
            )
            .await?;

        Ok(schema_transformation)
    }
}

async fn get_basis_node<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    group: Vec<Arc<Context>>,
    options: &Options,
    stage_context: &StageContext,
    document_summary: &str,
) -> Result<BasisNode, Errors> {
    log::trace!("In get_basis_node");

    let first = group.first().unwrap();

    if first.data_node.fields.is_empty() {
        return Err(Errors::InsufficientPrerequisites(
            "get_basis_node called with empty fields group".to_string(),
        ));
    }

    let basis_lineage = first.basis_lineage().ok_or_else(|| {
        Errors::InsufficientPrerequisites("basis_lineage not set on context".to_string())
    })?;

    let data_node = &first.data_node;
    let hash = data_node.hash.clone();
    let description = data_node.description.clone();

    if !options.regenerate {
        if let Some(basis_node) = provider.get_basis_node_by_lineage(&basis_lineage).await? {
            return Ok(basis_node);
        };
    }

    let (field_transformations, (tokens,)) = LLM::get_node_transformations(
        group,
        Arc::clone(&meta_context),
        document_summary,
    ).await?;

    let basis_node = BasisNode {
        id: ID::new(),
        hash,
        description,
        lineage: basis_lineage.clone(),
        transformations: field_transformations,
    };

    provider
        .save_basis_node(&basis_lineage, basis_node.clone())
        .await?;

    stage_context.record_events("Node analysis", tokens);

    Ok(basis_node)
}

async fn get_context_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    options: &Options,
) -> Result<Vec<Vec<Arc<Context>>>, Errors> {
    let contexts = {
        let lock = read_lock!(meta_context);
        lock.contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Contexts not provided in meta context".to_string())
            })?
    };

    let mut filtered_contexts: Vec<Arc<Context>> = Vec::new();
    let mut empty_field_contexts: Vec<Arc<Context>> = Vec::new();

    for context in contexts.values() {
        if context.data_node.fields.is_empty() {
            empty_field_contexts.push(context.clone());
        } else {
            filtered_contexts.push(context.clone());
        }
    }

    // Empty-field contexts are intentionally skipped — they produce no BasisNode and require no
    // provider lookup or LLM interpretation.
    log::debug!("Skipping {} contexts with empty fields", empty_field_contexts.len());

    let mut acyclic_contexts: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();

    for context in filtered_contexts {
        acyclic_contexts
            .entry(context.acyclic_lineage.clone())
            .or_insert_with(Vec::new)
            .push(context.clone());
    }

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut context_groups: Vec<Vec<Arc<Context>>> = Vec::new();
    let mut handles = Vec::new();

    for (acyclic_lineage, contexts_in_group) in acyclic_contexts {
        let mut lineage_subgroups: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
        for context in &contexts_in_group {
            lineage_subgroups
                .entry(context.lineage.clone())
                .or_insert_with(Vec::new)
                .push(context.clone());
        }

        if lineage_subgroups.len() == 1 {
            let (_, subgroup) = lineage_subgroups.into_iter().next().unwrap();

            if !options.regenerate {
                let bl = compute_basis_lineage(&acyclic_lineage, None, None);
                if provider.get_basis_node_by_lineage(&bl).await?.is_some() {
                    for context in &subgroup {
                        *context.basis_lineage.write().unwrap() = Some(bl.clone());
                    }
                    context_groups.push(subgroup);
                    continue;
                }
            }

            let has_diverging = if !subgroup.is_empty() {
                let mut common = read_lock!(subgroup[0].indexed_lineages).clone();
                for context in &subgroup[1..] {
                    let il = read_lock!(context.indexed_lineages);
                    common.retain(|l| il.contains(l));
                }
                subgroup.iter().any(|context| {
                    let il = read_lock!(context.indexed_lineages);
                    il.iter().any(|l| !common.contains(l))
                })
            } else {
                false
            };

            if !has_diverging {
                for context in &subgroup {
                    *context.basis_lineage.write().unwrap() = Some(acyclic_lineage.clone());
                }
                context_groups.push(subgroup);
            }
        } else {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let cloned_acyclic_lineage = acyclic_lineage.clone();
            let cloned_lineage_subgroups = lineage_subgroups.clone();
            let cloned_options = options.clone();

            let handle = task::spawn(async move {
                let _permit = permit;
                get_acyclic_context_groups(cloned_provider, cloned_meta_context, cloned_acyclic_lineage, &cloned_lineage_subgroups, &cloned_options).await
            });

            handles.push(handle);
        }
    }

    let results: Vec<Result<Vec<Vec<Arc<Context>>>, Errors>> = try_join_all(handles).await?;
    for result in results {
        context_groups.extend(result?);
    }

    Ok(context_groups)
}

fn compute_basis_lineage(acyclic_lineage: &Lineage, lineage: Option<&Lineage>, discriminator: Option<&Lineage>) -> Lineage {
    let mut key = acyclic_lineage.clone();
    if let Some(l) = lineage {
        key = key.with_hash(Hash::from_str(&l.to_string()));
    }
    if let Some(d) = discriminator {
        key = key.with_hash(Hash::from_str(&d.to_string()));
    }
    key
}

async fn get_acyclic_context_groups<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    acyclic_lineage: Lineage,
    lineage_subgroups: &HashMap<Lineage, Vec<Arc<Context>>>,
    options: &Options,
) -> Result<Vec<Vec<Arc<Context>>>, Errors> {
    if !options.regenerate {
        // Probe Acyclic: single BasisNode for all lineages under this acyclic_lineage
        let acyclic_bl = compute_basis_lineage(&acyclic_lineage, None, None);
        if provider.get_basis_node_by_lineage(&acyclic_bl).await?.is_some() {
            let all_contexts: Vec<Arc<Context>> = lineage_subgroups.values().flatten().cloned().collect();
            for context in &all_contexts {
                *context.basis_lineage.write().unwrap() = Some(acyclic_bl.clone());
            }
            return Ok(vec![all_contexts]);
        }

        // Probe per-lineage: each lineage is independently Uniform or Diverging.
        // Writes are deferred until every lineage classifies, so a partial failure
        // doesn't leak half-written basis_lineage state into the LLM fallback.
        let mut per_lineage_groups: Vec<Vec<Arc<Context>>> = Vec::new();
        let mut pending_writes: Vec<(Arc<Context>, Lineage)> = Vec::new();
        let mut all_classified = true;
        'outer: for (lineage, subgroup) in lineage_subgroups {
            let uniform_bl = compute_basis_lineage(&acyclic_lineage, Some(lineage), None);
            if provider.get_basis_node_by_lineage(&uniform_bl).await?.is_some() {
                for context in subgroup {
                    pending_writes.push((context.clone(), uniform_bl.clone()));
                }
                per_lineage_groups.push(subgroup.clone());
                continue;
            }

            let mut subgroups_by_discriminator: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
            for context in subgroup {
                let il = read_lock!(context.indexed_lineages).clone();
                let mut matched = false;
                for indexed_lineage in &il {
                    let bl = compute_basis_lineage(&acyclic_lineage, Some(lineage), Some(indexed_lineage));
                    if provider.get_basis_node_by_lineage(&bl).await?.is_some() {
                        subgroups_by_discriminator
                            .entry(bl)
                            .or_insert_with(Vec::new)
                            .push(context.clone());
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    all_classified = false;
                    break 'outer;
                }
            }
            for (bl, contexts) in subgroups_by_discriminator {
                for context in &contexts {
                    pending_writes.push((context.clone(), bl.clone()));
                }
                per_lineage_groups.push(contexts);
            }
        }
        if all_classified && !per_lineage_groups.is_empty() {
            for (context, bl) in pending_writes {
                *context.basis_lineage.write().unwrap() = Some(bl);
            }
            return Ok(per_lineage_groups);
        }
    }

    let (node_groups, _tokens) = LLM::get_node_groups(
        Arc::clone(&meta_context),
        acyclic_lineage.clone(),
        lineage_subgroups,
    ).await?;

    let mut context_groups: Vec<Vec<Arc<Context>>> = Vec::new();

    let is_acyclic = node_groups.values().any(|c| matches!(c, NodeGroupClassification::Acyclic));

    if is_acyclic {
        let all_contexts: Vec<Arc<Context>> = lineage_subgroups.values().flatten().cloned().collect();
        let bl = compute_basis_lineage(&acyclic_lineage, None, None);
        for context in &all_contexts {
            *context.basis_lineage.write().unwrap() = Some(bl.clone());
        }
        context_groups.push(all_contexts);
    } else {
        for (lineage, classification) in &node_groups {
            let subgroup = lineage_subgroups.get(lineage).map(|v| v.as_slice()).unwrap_or(&[]);

            match classification {
                NodeGroupClassification::Acyclic => unreachable!(),
                NodeGroupClassification::Uniform => {
                    let bl = compute_basis_lineage(&acyclic_lineage, Some(lineage), None);
                    for context in subgroup {
                        *context.basis_lineage.write().unwrap() = Some(bl.clone());
                    }
                    context_groups.push(subgroup.to_vec());
                }
                NodeGroupClassification::Diverging(discriminators) => {
                    for discriminator in discriminators {
                        let matching_contexts: Vec<Arc<Context>> = subgroup
                            .iter()
                            .filter(|context| read_lock!(context.indexed_lineages).contains(discriminator))
                            .cloned()
                            .collect();

                        if matching_contexts.is_empty() {
                            continue;
                        }

                        let bl = compute_basis_lineage(&acyclic_lineage, Some(lineage), Some(discriminator));
                        for context in &matching_contexts {
                            *context.basis_lineage.write().unwrap() = Some(bl.clone());
                        }
                        context_groups.push(matching_contexts);
                    }
                }
            }
        }
    }

    Ok(context_groups)
}
