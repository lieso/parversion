use std::sync::{Arc, RwLock};
use tokio::task;
use tokio::sync::Semaphore;
use futures::future::try_join_all;
use std::collections::HashMap;

use crate::prelude::*;
use crate::basis_node::BasisNode;
use crate::provider::Provider;
use crate::config::{CONFIG};
use crate::context_group::ContextGroup;
use crate::llm::LLM;
use crate::meta_context::MetaContext;
use crate::transformation::{
    FieldTransformation,
    SchemaTransformation
};
use crate::schema_context::SchemaContext;
use crate::graph_node::Graph;
use crate::path::Path;

pub async fn get_translation_schema_transformations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<Lineage, Arc<SchemaTransformation>>, Errors> {
    log::trace!("In get_translation_schema_transformations");

    let schema_contexts: Vec<Arc<SchemaContext>> = {
        let lock = read_lock!(meta_context);

        lock.schema_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Schema contexts not provided in meta context".to_string()
                )
            })?
            .values()
            .cloned()
            .collect()
    };
    let target_schema: String = {
        let lock = read_lock!(meta_context);

        let graph_root = lock.translation_schema_graph_root
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Translation schema graph root not provided in meta context".to_string()
                )
            })?
            .clone();

        let schema_contexts_map: HashMap<ID, Arc<SchemaContext>> = {
            let lock = read_lock!(meta_context);
            lock.translation_schema_contexts
                .clone()
                .unwrap()
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
            ).await?;

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

            let handle = task::spawn(async move {
                let _permit = permit;
                let transformation = get_translation_schema_transformation(
                    cloned_provider,
                    cloned_meta_context,
                    schema_context.clone(),
                    cloned_target_schema,
                ).await?;

                Ok((transformation.lineage.clone(), Arc::new(transformation)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(Lineage, Arc<SchemaTransformation>), Errors>> = try_join_all(handles).await?;

        let hashmap_results: HashMap<Lineage, Arc<SchemaTransformation>> = results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

pub async fn get_normal_schema_transformations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>
) -> Result<HashMap<Lineage, Arc<SchemaTransformation>>, Errors> {
    log::trace!("In get_normal_schema_transformations");

    let schema_contexts: Vec<Arc<SchemaContext>> = {
        let lock = read_lock!(meta_context);

        lock.schema_contexts
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Normal schema contexts not provided in meta context".to_string()
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
                schema_context.clone()
            ).await?;

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

            let handle = task::spawn(async move {
                let _permit = permit;
                let transformation = get_normal_schema_transformation(
                    cloned_provider,
                    cloned_meta_context,
                    schema_context.clone()
                ).await?;

                Ok((transformation.lineage.clone(), Arc::new(transformation)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(Lineage, Arc<SchemaTransformation>), Errors>> = try_join_all(handles).await?;

        let hashmap_results: HashMap<Lineage, Arc<SchemaTransformation>> = results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

pub async fn get_basis_nodes<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In get_basis_nodes");

    let context_groups = ContextGroup::from_meta_context(Arc::clone(&meta_context));

    log::info!("Number of context groups: {}", context_groups.len());

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

    if max_concurrency == 1 {
        let mut results = HashMap::new();

        for context_group in context_groups {
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let result = get_basis_node(
                cloned_provider,
                cloned_meta_context,
                context_group.clone()
            ).await?;

            results.insert(result.id.clone(), Arc::new(result));
        }

        Ok(results)
    } else {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for context_group in context_groups {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);

            let handle = task::spawn(async move {
                let _permit = permit;
                let basis_node = get_basis_node(
                    cloned_provider,
                    cloned_meta_context,
                    context_group.clone()
                ).await?;

                Ok((basis_node.id.clone(), Arc::new(basis_node)))
            });
            handles.push(handle);
        }

        let results: Vec<Result<(ID, Arc<BasisNode>), Errors>> = try_join_all(handles).await?;

        let hashmap_results: HashMap<ID, Arc<BasisNode>> = results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

async fn get_normal_schema_transformation<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    schema_context: Arc<SchemaContext>,
) -> Result<SchemaTransformation, Errors> {
    log::trace!("In get_normal_schema_transformation");

    let lineage = &schema_context.lineage;

    if let Some(schema_transformation) = provider.get_schema_transformation(&lineage, None).await? {
        log::info!("Provider has supplied normal schema transformation");

        return Ok(schema_transformation);
    }

    let snippet = schema_context.generate_snippet(Arc::clone(&meta_context));

    let (key, description, _aliases, path) = LLM::get_normal_schema(&snippet).await?;

    let schema_transformation = SchemaTransformation {
        id: ID::new(),
        description,
        key,
        path,
        lineage: lineage.clone(),
        subgraph_hash: None,
    };

    provider.save_schema_transformation(
        &lineage,
        None,
        schema_transformation.clone(),
    ).await?;

    Ok(schema_transformation)
}

async fn get_translation_schema_transformation<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    schema_context: Arc<SchemaContext>,
    target_schema: Arc<String>,
) -> Result<SchemaTransformation, Errors> {
    log::trace!("In get_translation_schema_transformation");

    let lineage = &schema_context.lineage;
    let schema_root: Graph = {
        let lock = read_lock!(meta_context);
        lock.translation_schema_graph_root
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Schema contexts not provided in meta context".to_string()
                )
            })?
    };
    let subgraph_hash: Hash = {
        let lock = read_lock!(schema_root);
        lock.subgraph_hash.clone()
    };

    if let Some(schema_transformation) = provider.get_schema_transformation(&lineage, Some(&subgraph_hash)).await? {
        log::info!("Provider has supplied translation schema transformtion"); 

        return Ok(schema_transformation);
    }

    let snippet = schema_context.generate_snippet(Arc::clone(&meta_context));

    let result = LLM::get_translation_schema(
        Arc::clone(&meta_context),
        &snippet,
        Arc::clone(&target_schema)
    ).await?;

    if let Some((key, description, path)) = result {
        let schema_transformation = SchemaTransformation {
            id: ID::new(),
            description,
            key,
            path,
            lineage: lineage.clone(),
            subgraph_hash: Some(subgraph_hash.clone()),
        };

        provider.save_schema_transformation(
            &lineage,
            Some(&subgraph_hash),
            schema_transformation.clone(),
        ).await?;

        Ok(schema_transformation)
    } else {
        let schema_transformation = SchemaTransformation {
            id: ID::new(),
            description: schema_context.schema_node.description.clone(),
            key: schema_context.schema_node.name.clone(),
            path: Path::new(),
            lineage: lineage.clone(),
            subgraph_hash: Some(subgraph_hash.clone()),
        };

        provider.save_schema_transformation(
            &lineage,
            Some(&subgraph_hash),
            schema_transformation.clone(),
        ).await?;

        Ok(schema_transformation)
    }
}

async fn get_basis_node<P: Provider>(
    provider: Arc<P>,
    _meta_context: Arc<RwLock<MetaContext>>,
    context_group: ContextGroup,
) -> Result<BasisNode, Errors> {
    log::trace!("In get_basis_node");

    let lineage = &context_group.lineage.clone();
    let data_node = &context_group.contexts.first().unwrap().data_node.clone();
    let hash = data_node.hash.clone();
    let description = data_node.description.clone();

    if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
        log::info!("Provider has supplied basis node");

        return Ok(basis_node);
    };

    let field_transformations: Vec<FieldTransformation> = LLM::get_field_transformations(
        context_group.clone()
    ).await?;

    log::info!("Obtained field transformation");

    let basis_node = BasisNode {
        id: ID::new(),
        hash,
        description,
        lineage: lineage.clone(),
        transformations: field_transformations,
    };

    provider.save_basis_node(
        &lineage,
        basis_node.clone(),
    ).await?;

    Ok(basis_node)
}
