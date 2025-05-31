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
};

pub async fn get_basis_nodes<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<ID, Arc<BasisNode>>, Errors> {
    log::trace!("In get_basis_nodes");

    let context_groups = ContextGroup::from_meta_context(Arc::clone(&meta_context));

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
