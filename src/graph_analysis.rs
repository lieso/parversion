use std::sync::{Arc, RwLock, Mutex};
use futures::future::try_join_all;
use tokio::task;
use std::collections::{HashMap, HashSet};
use tokio::sync::Semaphore;

use crate::prelude::*;
use crate::basis_cluster::{BasisCluster, BasisClusterMetadata, NetworkRelationship, NetworkTraversal};
use crate::basis_network::BasisNetwork;
use crate::reasoner::ReasonerMetadata;

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

pub async fn generate_network_traversals<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<NetworkTraversal>>, Errors> {
    log::trace!("In generate_network_traversals");

    unimplemented!()
}

pub async fn generate_basis_clusters<P: Provider, R: Reasoner>(
    provider: Arc<P>,
    reasoner: Arc<R>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    options: &Options,
    stage_context: &StageContext
) -> Result<HashMap<ID, Arc<BasisCluster>>, Errors> {
    log::trace!("In generate_basis_clusters");

    let mut basis_networks: Vec<Arc<BasisNetwork>> = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
            })?
            .into_values()
            .collect()
    };

    // sort to increase chance of Provider hit
    basis_networks.sort_by(|a, b| a.basis_lineages.to_string().cmp(&b.basis_lineages.to_string()));

    let max_concurrency = basis_networks.len().clamp(4, 16);
    let semaphore = Arc::new(Semaphore::new(max_concurrency));

    let union_find = Arc::new(Mutex::new(UnionFind::new()));
    let mut handles = Vec::new();

    for i in 0..basis_networks.len() {
        for j in (i + 1)..basis_networks.len() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cloned_provider = Arc::clone(&provider);
            let cloned_reasoner = Arc::clone(&reasoner);
            let cloned_normalization_context = Arc::clone(&normalization_context);
            let cloned_stage_context = stage_context.clone();
            let cloned_options = options.clone();
            let cloned_union_find = Arc::clone(&union_find);
            let left = Arc::clone(&basis_networks[i]);
            let right = Arc::clone(&basis_networks[j]);

            handles.push(task::spawn(async move {
                let _permit = permit;
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

    let results: Vec<(NetworkRelationship, Option<ReasonerMetadata>)> = try_join_all(handles).await?
        .into_iter()
        .collect::<Result<Vec<_>, Errors>>()?
        .into_iter()
        .filter_map(|(relationship, metadata)| relationship.map(|r| (r, metadata)))
        .collect();

    let mut union_find = union_find.lock().unwrap();

    let mut cluster_networks: HashMap<Hash, HashSet<Hash>> = HashMap::new();
    for network in &basis_networks {
        let root = union_find.find(&network.basis_lineages);
        cluster_networks.entry(root).or_default().insert(network.basis_lineages.clone());
    }

    let mut cluster_relationships: HashMap<Hash, Vec<NetworkRelationship>> = HashMap::new();
    let mut cluster_prompts: HashMap<Hash, Vec<Hash>> = HashMap::new();

    for (relationship, metadata) in results {
        let root = union_find.find(&relationship.from);
        cluster_relationships.entry(root.clone()).or_default().push(relationship);
        if let Some(metadata) = metadata {
            cluster_prompts.entry(root).or_default().push(metadata.prompt_hash);
        }
    }

    let basis_clusters: HashMap<ID, Arc<BasisCluster>> = cluster_networks
        .into_iter()
        .map(|(root, networks)| {
            let basis_cluster = BasisCluster {
                id: ID::new(),
                networks,
                relationships: cluster_relationships.remove(&root).unwrap_or_default(),
                metadata: BasisClusterMetadata {
                    prompts: cluster_prompts.remove(&root).unwrap_or_default(),
                }
            };

            (basis_cluster.id.clone(), Arc::new(basis_cluster))
        })
        .collect();

    Ok(basis_clusters)
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
) -> Result<(Option<NetworkRelationship>, Option<ReasonerMetadata>), Errors> {
    stage_context.record_events("Cluster analysis", 0);

    let from = left.basis_lineages.clone();
    let to = right.basis_lineages.clone();

    {
        let mut union_find = union_find.lock().unwrap();
        if union_find.same_set(&from, &to) {
            return Ok((None, None));
        }
    }

    let relationship = if !options.regenerate {
        provider.get_network_relationship(left.clone(), right.clone()).await?
    } else {
        None
    };

    let (relationship, metadata) = match relationship {
        Some(relationship) => (relationship, None),
        None => {
            let (relationship, metadata) = reasoner.network_relationship(
                Arc::clone(&normalization_context),
                Arc::clone(&left),
                Arc::clone(&right),
            ).await?;

            stage_context.record_events("Cluster analysis", metadata.tokens.into());

            provider
                .save_network_relationship(left.clone(), right.clone(), relationship.clone())
                .await?;

            (relationship, Some(metadata))
        }
    };

    if relationship.is_some() {
        let mut union_find = union_find.lock().unwrap();
        union_find.union(&from, &to);
    }

    Ok((relationship, metadata))
}
