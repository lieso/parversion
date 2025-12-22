use std::sync::{Arc, RwLock};
use tokio::task;
use tokio::sync::Semaphore;
use futures::future::try_join_all;
use std::collections::HashMap;

use crate::prelude::*;
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::operation::Operation;
use crate::function::Function;
use crate::config::{CONFIG};
use crate::llm::LLM;

pub async fn functions_to_operations<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<HashMap<Hash, Arc<Operation>>, Errors> {
    log::trace!("In functions_to_operations");

    let functions: Vec<Function> = {
        let lock = read_lock!(meta_context);

        lock.functions
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError(
                    "Missings functions from meta context".to_string()
                )
            })?
    };

    let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;

    if max_concurrency == 1 {
        let mut results = HashMap::new();

        for function in functions.iter() {
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            let result = function_to_operation(
                cloned_provider,
                cloned_meta_context,
                function.clone(),
            ).await?;

            results.insert(function.hash.clone(), Arc::new(result));
        }
        
        Ok(results)
    } else {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for function in functions.into_iter() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_meta_context = Arc::clone(&meta_context);
            
            let handle = task::spawn(async move {
                let _permit = permit;
                let result = function_to_operation(
                    cloned_provider,
                    cloned_meta_context,
                    function.clone(),
                ).await?;

                Ok((function.hash.clone(), Arc::new(result)))
            });

            handles.push(handle);
        }

        let results: Vec<Result<(Hash, Arc<Operation>), Errors>> = try_join_all(handles).await?;

        let hashmap_results: HashMap<Hash, Arc<Operation>> = results.into_iter().collect::<Result<_, _>>()?;

        Ok(hashmap_results)
    }
}

async fn function_to_operation<P: Provider>(
    provider: Arc<P>,
    meta_context: Arc<RwLock<MetaContext>>,
    function: Function
) -> Result<Operation, Errors> {
    log::trace!("In function_to_operation");
    
    let hash: &Hash = &function.hash;

    if let Some(operation) = provider.get_operation_by_hash(hash).await? {
        log::info!("Provider has supplied operation");

        return Ok(operation);
    }

    if let Some(something) = LLM::function_to_operation(&function.code).await? {
        unimplemented!()
    } else {
        let operation = Operation::new(hash);

        provider.save_operation(
            hash,
            operation.clone()
        ).await?;

        Ok(operation)
    }
}
