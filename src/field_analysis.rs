use tokio::task;
use futures::future::try_join_all;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::basis_field::BasisField;

pub async fn get_basis_fields<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<HashMap<ID, Arc<BasisField>>, Errors> {
    log::trace!("In get_basis_fields");

    stage_context.record_events("Field analysis", 0);

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().ok_or(Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string()))?
    };

    if !options.regenerate {
        let basis_fields: Vec<BasisField> = provider
            .get_basis_fields_by_acyclic_subgraph_hash(&meta_context.acyclic_subgraph_hash).await?
            .into_iter()
            .collect();

        if !basis_fields.is_empty() {
            let field_map: HashMap<ID, Arc<BasisField>> = basis_fields.into_iter()
                .map(|basis_field| {
                    let basis_field = Arc::new(basis_field);
                    let id = basis_field.id.clone();
                    (id, basis_field)
                })
                .collect();

            return Ok(field_map);
        }
    }

    let contexts: Vec<Arc<Context>> = meta_context.contexts.values().cloned().collect();
    log::info!("Number of contexts: {}", contexts.len());

    let mut contexts_by_field: HashMap<String, Vec<Arc<Context>>> = HashMap::new();
    for context in contexts {
        for field_name in context.data_node.fields.keys() {
            if field_name == "text" {
                continue;
            }

            contexts_by_field
                .entry(field_name.clone())
                .or_insert_with(Vec::new)
                .push(Arc::clone(&context));
        }
    }
    log::info!("Number of field groups: {}", contexts_by_field.len());

    let mut handles = Vec::new();

    for (field, contexts_in_group) in contexts_by_field {
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();

        let handle = task::spawn(async move {
            let result = cloned_reasoner.basis_field(
                cloned_normalization_context,
                contexts_in_group,
                field
            ).await;

            match result {
                Ok((maybe_basis_field, metadata)) => {
                    cloned_stage_context.record_events("Field analysis", metadata.tokens.into());
                    Ok(maybe_basis_field)
                }
                Err(e) => Err(e),
            }
        });

        handles.push(handle);
    }

    let results: Vec<Result<Option<BasisField>, Errors>> = try_join_all(handles).await?;

    let mut basis_fields: Vec<BasisField> = results.into_iter()
        .filter_map(|res| {
            match res {
                Ok(Some(basis_field)) => Some(Ok(basis_field)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<BasisField>, Errors>>()?;

    // Always ensure 'text' is a basis field
    basis_fields.push(BasisField {
        id: ID::new(),
        acyclic_subgraph_hash: meta_context.acyclic_subgraph_hash.clone(),
        name: "text".to_string()
    });

    provider.save_basis_fields(
        &meta_context.acyclic_subgraph_hash,
        basis_fields.clone()
    ).await?;

    let field_map: HashMap<ID, Arc<BasisField>> = basis_fields.into_iter()
        .map(|basis_field| {
            let basis_field = Arc::new(basis_field);
            let id = basis_field.id.clone();
            (id, basis_field)
        })
        .collect();

    Ok(field_map)
}
