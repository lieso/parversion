use futures::future::try_join_all;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::task;

use crate::basis_graph::{BasisGraph, NetworkRelationship};
use crate::basis_network::BasisNetwork;
use crate::prelude::*;

pub async fn generate_basis_graph<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
) -> Result<Arc<BasisGraph>, Errors> {
    let basis_networks = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(
                    "Basis networks not provided in normalization context".to_string(),
                )
            })?
            .clone()
    };

    let basis_networks: Vec<Arc<BasisNetwork>> = basis_networks.values().cloned().collect();

    let mut handles = Vec::new();

    for basis_network in basis_networks {
        let cloned_provider = Arc::clone(&provider);
        let cloned_reasoner = Arc::clone(&reasoner);
        let cloned_normalization_context = Arc::clone(&normalization_context);
        let cloned_stage_context = stage_context.clone();
        let cloned_options = options.clone();

        let handle = task::spawn(async move {
            generate_network_relationship(
                cloned_provider,
                cloned_reasoner,
                cloned_normalization_context,
                &cloned_options,
                &cloned_stage_context,
                basis_network.clone(),
                basis_network.clone(),
            )
            .await
        });

        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    for result in results {
        let relationship = result?;
        log::debug!("relationship: {:?}", relationship);
    }

    unimplemented!()
}

async fn generate_network_relationship<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<NetworkRelationship, Errors> {
    stage_context.record_events("Network relationship", 0);

    if !options.regenerate {
        if let Some(network_relationship) = provider
            .get_network_relationship(left.clone(), right.clone())
            .await?
        {
            return Ok(network_relationship);
        }
    }

    let (network_relationship, metadata) = reasoner
        .network_relationship(
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone(),
        )
        .await?;

    stage_context.record_events("Network relationship", metadata.tokens.into());

    provider
        .save_network_relationship(left.clone(), right.clone(), network_relationship.clone())
        .await?;

    Ok(network_relationship)
}
