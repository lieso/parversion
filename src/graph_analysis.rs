use std::sync::{Arc, RwLock, Mutex};
use futures::future::try_join_all;
use tokio::task;
use std::collections::HashMap;

use crate::prelude::*;
use crate::basis_cluster::{BasisCluster, NetworkRelationship};
use crate::basis_network::BasisNetwork;

struct UnionFind {
    parent: HashMap<Hash, Hash>,
}

impl UnionFind {
    fn new() -> Self {
        UnionFind { parent: HashMap::new() }
    }

    fn find(&mut self, x: &Hash) -> Hash {
        let parent = self.parent.entry(x.clone()).or_insert_with(|| x.clone()).clone();
        if &parent == x {
            return x.clone();
        }
        let root = self.find(&parent);
        self.parent.insert(x.clone(), root.clone());
        root
    }

    fn union(&mut self, a: &Hash, b: &Hash) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_a, root_b);
        }
    }

    fn same_set(&mut self, a: &Hash, b: &Hash) -> bool {
        self.find(a) == self.find(b)
    }
}

pub async fn generate_basis_clusters<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisCluster>>, Errors> {
    log::trace!("In generate_basis_clusters");

    let basis_networks = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
            })?
            .into_values()
            .take(2)
            .collect()
    };

    let union_find = Arc::new(Mutex::new(UnionFind::new()));
    let mut handles = Vec::new();

    for i in 0..basis_networks.len() {
        for j in (i + 1)..basis_networks.len() {
            let cloned_provider = Arc::clone(&provider);
            let cloned_reasoner = Arc::clone(&reasoner);
            let cloned_normalization_context = Arc::clone(&normalization_context);
            let cloned_stage_context = stage_context.clone();
            let cloned_options = options.clone();
            let cloned_union_find = Arc::clone(&union_find);
            let left = Arc::clone(&basis_networks[i]);
            let right = Arc::clone(&basis_networks[j]);

            handles.push(task::spawn(async move {
                generate_network_relationship(
                    cloned_provider,
                    cloned_reasoner,
                    cloned_normalization_context,
                    &cloned_options,
                    &cloned_stage_context,
                    cloned_union_find,
                    left,
                    right,
                ).await
            }));
        }
    }

    let relationships: Vec<NetworkRelationship> = try_join_all(handles).await?
        .into_iter()
        .collect::<Result<Vec<_>, Errors>>()?
        .into_iter()
        .flatten()
        .collect();

    unimplemented!()
}

async fn generate_network_relationship<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext,
    union_find: Arc<Mutex<UnionFind>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<Option<NetworkRelationship>, Errors> {
    stage_context.record_events("Cluster analysis", 0);

    let from = left.basis_lineages.clone();
    let to = right.basis_lineages.clone();

    {
        let mut union_find = union_find.lock().unwrap();
        if union_find.same_set(&from, &to) {
            return Ok(None);
        }
    }

    let (relationship, metadata) = reasoner.network_relationship(
        Arc::clone(&normalization_context),
        Arc::clone(&left),
        Arc::clone(&right),
    ).await?;

    stage_context.record_events("Cluster analysis", metadata.tokens.into());


    if relationship.is_some() {
        let mut union_find = union_find.lock().unwrap();
        union_find.union(&from, &to);
    }

    Ok(relationship)
}
