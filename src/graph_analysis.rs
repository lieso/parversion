use std::sync::{Arc, RwLock};
use futures::future::try_join_all;
use tokio::task;
use std::collections::HashMap;

use crate::prelude::*;
use crate::basis_cluster::BasisCluster;

pub async fn generate_basis_clusters<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisCluster>>, Errors> {
    log::trace!("In generate_basis_clusters");

    //let basis_networks = {
    //    let lock = read_lock!(normalization_context);
    //    lock.basis_networks
    //        .clone()
    //        .ok_or_else(|| {
    //            Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
    //        })?
    //};

    //let mut items: Vec<Arc<NetworkRelationship>> = basis_networks
    //    .into_values()
    //    .map(NetworkRelationship::Leaf)
    //    .map(Arc::new)
    //    .take(2)
    //    .collect();

    //while items.len() > 1 {
    //    let mut handles = Vec::new();
    //    let mut carry_over = None;

    //    let mut iter = items.into_iter();

    //    while let Some(first) = iter.next() {
    //        match iter.next() {
    //            Some(second) => {
    //                let cloned_provider = Arc::clone(&provider);
    //                let cloned_reasoner = Arc::clone(&reasoner);
    //                let cloned_normalization_context = Arc::clone(&normalization_context);
    //                let cloned_stage_context = stage_context.clone();
    //                let cloned_options = options.clone();

    //                let handle = task::spawn(async move {
    //                    generate_relationship(
    //                        cloned_provider,
    //                        cloned_reasoner,
    //                        cloned_normalization_context,
    //                        &cloned_options,
    //                        &cloned_stage_context,
    //                        first,
    //                        second,
    //                    ).await
    //                });

    //                handles.push(handle);
    //            }
    //            None => carry_over = Some(first),
    //        }
    //    }

    //    let mut next_items: Vec<Arc<NetworkRelationship>> = try_join_all(handles).await?
    //        .into_iter()
    //        .collect::<Result<Vec<_>, Errors>>()?;

    //    if let Some(leftover) = carry_over {
    //        next_items.push(leftover);
    //    }

    //    items = next_items;
    //}

    //let relationships = items.into_iter().next().ok_or_else(|| {
    //    Errors::UnexpectedError("Failed to generate basis graph: no items remaining after analysis".to_string())
    //});

    unimplemented!()
}
