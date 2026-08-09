use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::document::Document;
use crate::document::DocumentType;
use crate::document_format::DocumentFormat;
use crate::graph_node::{Graph, GraphNode};
use crate::group_analysis::resolve_context_groups;
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::provider::Provider;

const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

pub async fn report_basis_fields<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let basis_fields = {
        let lock = read_lock!(normalization_context);
        lock.basis_fields
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis fields not provided in normalization context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Meta context not provided in normalization context".to_string())
            })?
            .clone()
    };

    println!("{}=== Basis Field Report ({} fields) ==={}", CYAN, basis_fields.len(), RESET);
    println!("{}Total contexts analyzed: {}{}", CYAN, meta_context.contexts.len(), RESET);

    for field in &basis_fields {
        let contexts_with_field: usize = meta_context
            .contexts
            .values()
            .filter(|ctx| ctx.data_node.fields.contains_key(&field.name))
            .count();

        let percentage = if meta_context.contexts.is_empty() {
            0.0
        } else {
            (contexts_with_field as f64 / meta_context.contexts.len() as f64) * 100.0
        };

        println!("{}{}{}", CYAN, "-----------------------------------------------------------------------------------------------------", RESET);
        println!("{}--- Field [{}] ---{}", CYAN, field.name, RESET);
        println!("{}  id: {}{}", CYAN, field.id.to_string(), RESET);
        println!("{}  contexts with field: {} / {} ({:.1}%){}", CYAN, contexts_with_field, meta_context.contexts.len(), percentage, RESET);
        println!("{}  subgraph_hash: {}{}", CYAN, field.acyclic_subgraph_hash, RESET);
        println!("{}  prompts: {:?}{}", CYAN, field.metadata.prompts, RESET);
        println!("{}{}{}", CYAN, "-----------------------------------------------------------------------------------------------------", RESET);
    }

    println!();
    println!("{}=== End Basis Field Report ==={}", CYAN, RESET);

    Ok(())
}

pub async fn report_basis_groups<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let context_groups = {
        let lock = read_lock!(normalization_context);
        lock.context_groups
            .clone()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Context groups not provided in meta context".to_string())
            })?
    };
    let basis_groups = {
        let lock = read_lock!(normalization_context);
        lock.basis_groups
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis groups not provided in meta context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    println!("{}=== Basis Group Report ({} groups) ==={}", MAGENTA, basis_groups.len(), RESET);

    for group in &basis_groups {
        let acyclic = group.acyclic_lineage.to_string();
        let lineage_desc = match (&group.lineage, &group.indexed_lineage) {
            (Some(l), Some(il)) => format!(
                "acyclic={} lineage={} indexed_lineage={}",
                acyclic,
                l.to_string(),
                il.to_string()
            ),
            (Some(l), None) => format!("acyclic={} lineage={}", acyclic, l.to_string()),
            (None, _) => format!("acyclic={}", acyclic),
        };

        let contexts = context_groups.get(&group.id).map(|v| v.as_slice()).unwrap_or(&[]);

        println!("{}{}{}", MAGENTA, "-----------------------------------------------------------------------------------------------------", RESET);
        println!("{}--- Group [{}] ---{}", MAGENTA, lineage_desc, RESET);
        println!("{}  total contexts: {}{}", MAGENTA, contexts.len(), RESET);
        println!("{}  prompts: {:?}{}", MAGENTA, group.metadata.prompts, RESET);
        println!("{}{}{}", MAGENTA, "-----------------------------------------------------------------------------------------------------", RESET);

        for (i, context) in contexts.iter().take(10).enumerate() {
            let fields: Vec<String> = context
                .data_node
                .fields
                .iter()
                .map(|(k, v)| format!("{}={:?}", k, v))
                .collect();
            println!("{}  [{}] {}{}", MAGENTA, i + 1, fields.join(", "), RESET);
        }

        println!();
    }

    println!("{}=== End Basis Group Report ==={}", MAGENTA, RESET);

    Ok(())
}

pub async fn report_basis_nodes<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let (context_groups, _context_to_group) = resolve_context_groups(
        Arc::clone(&normalization_context)
    )?;

    let basis_groups = {
        let lock = read_lock!(normalization_context);
        lock.basis_groups
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis groups not provided in normalization context".to_string())
            })?
            .clone()
    };

    let basis_nodes = {
        let lock = read_lock!(normalization_context);
        lock.basis_nodes
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis nodes not provided in normalization context".to_string())
            })?
            .clone()
    };

    let mut covered_nodes = std::collections::HashSet::new();

    println!("{}=== Basis Node Report ==={}", GREEN, RESET);

    for (group_id, contexts) in &context_groups {
        let basis_group = basis_groups.get(group_id).ok_or_else(|| {
            Errors::DeficientNormalizationContextError(format!("Basis group not found for id {}", group_id.to_string()))
        })?;

        let basis_lineage = basis_group.get_basis_lineage();

        let basis_node = basis_nodes
            .values()
            .find(|node| node.lineage == basis_lineage)
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError(format!("Basis node not found for lineage {}", basis_lineage.to_string()))
            })?;

        if covered_nodes.contains(&basis_node.id) {
            continue;
        }
        covered_nodes.insert(basis_node.id.clone());

        println!("{}{}{}", GREEN, "-----------------------------------------------------------------------------------------------------", RESET);
        println!("{}--- Node [{}] ---{}", GREEN, basis_node.id.to_string(), RESET);
        println!("{}  lineage: {}{}", GREEN, basis_node.lineage.to_string(), RESET);
        println!("{}  transformations: {} count{}", GREEN, basis_node.transformations.len(), RESET);
        println!("{}  prompts: {:?}{}", GREEN, basis_node.metadata.prompts, RESET);
        println!("{}{}{}", GREEN, "-----------------------------------------------------------------------------------------------------", RESET);

        let sample_contexts: Vec<_> = contexts.iter().take(3).collect();

        for (ctx_idx, context) in sample_contexts.iter().enumerate() {
            println!("{}  [Context {}]{}", GREEN, ctx_idx + 1, RESET);
            println!("{}    Before: {:?}{}", GREEN, context.data_node.fields, RESET);

            for (txn_idx, transformation) in basis_node.transformations.iter().enumerate() {
                println!("{}    [Transformation {}] {}{}", GREEN, txn_idx + 1, transformation.description, RESET);
                println!("{}      field: {}, image: {}{}", GREEN, transformation.field, transformation.image, RESET);

                match transformation.transform(Arc::clone(&context.data_node)) {
                    Ok(transformed) => {
                        println!("{}      After: {:?}{}", GREEN, transformed.fields, RESET);
                    }
                    Err(e) => {
                        println!("{}      Error: {:?}{}", GREEN, e, RESET);
                    }
                }
            }

            println!();
        }

        println!();
    }

    println!("{}=== End Basis Node Report ==={}", GREEN, RESET);

    Ok(())
}

pub async fn report_basis_networks<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let basis_networks = {
        let lock = read_lock!(normalization_context);
        lock.basis_networks
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    println!("{}=== Basis Network Report ({} networks) ==={}", YELLOW, basis_networks.len(), RESET);

    for network in &basis_networks {
        println!("{}{}{}", YELLOW, "-----------------------------------------------------------------------------------------------------", RESET);
        println!("{}--- Network [{}] ---{}", YELLOW, network.id.to_string(), RESET);
        println!("{}  basis_lineages: {}{}", YELLOW, network.basis_lineages, RESET);
        println!("{}  transformations: {} count{}", YELLOW, network.transformations.len(), RESET);

        for (txn_idx, transformation) in network.transformations.iter().enumerate() {
            println!("{}  [Transformation {}]{}", YELLOW, txn_idx + 1, RESET);
            println!("{}    image: {}{}", YELLOW, transformation.image, RESET);
            println!("{}    description: {}{}", YELLOW, transformation.description, RESET);
            println!("{}    keys: {:?}{}", YELLOW, transformation.keys, RESET);
        }

        println!("{}  prompts: {:?}{}", YELLOW, network.metadata.prompts, RESET);

        let contexts = {
            let lock = read_lock!(normalization_context);
            lock.basis_network_contexts
                .as_ref()
                .ok_or_else(|| {
                    Errors::DeficientNormalizationContextError("Basis network contexts not provided in normalization context".to_string())
                })?
                .get(&network.id)
                .map(|contexts| contexts.iter().take(3).cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        let graph_root: Graph = Arc::new(RwLock::new(GraphNode {
            id: ID::new(),
            parents: Vec::new(),
            description: String::new(),
            hash: Hash::new(),
            subgraph_hash: Hash::new(),
            lineage: Lineage::new(),
            children: Vec::new(),
        }));

        let normalized: Vec<NormalContext> = contexts
            .iter()
            .map(|context| {
                network.apply(
                    Arc::clone(&normalization_context),
                    context.clone(),
                    Arc::clone(&graph_root),
                )
            })
            .collect::<Result<Vec<NormalContext>, Errors>>()?;

        let mut normal_contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
        let mut contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();

        for normal_context in normalized {
            let normal_context = Arc::new(normal_context);

            normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
            contexts_lookup.insert(
                read_lock!(normal_context.graph_node).id.clone(),
                Arc::clone(&normal_context)
            );
        }

        let root_normal_context = Arc::new(NormalContext {
            id: ID::new(),
            network_name: None,
            network_description: None,
            data_node: Arc::new(DataNode {
                id: ID::new(),
                hash: Hash::new(),
                lineage: Lineage::new(),
                fields: HashMap::new(),
                description: String::new(),
            }),
            graph_node: Arc::clone(&graph_root),
            contexts: Vec::new(),
        });

        normal_contexts.insert(root_normal_context.id.clone(), Arc::clone(&root_normal_context));
        contexts_lookup.insert(
            read_lock!(root_normal_context.graph_node).id.clone(),
            Arc::clone(&root_normal_context)
        );

        let normal_meta_context = NormalMetaContext {
            contexts: normal_contexts,
            graph_root,
            contexts_lookup,
        };

        let document = Document::from_normal_meta_context(
            &normal_meta_context,
            &DocumentFormat {
                format_type: DocumentType::Json,
                encoding: Some(String::from("UTF-8")),
                indent: None,
                line_ending: None,
                headers: None,
                wrap_text: None,
                exclude_nulls: None,
                custom_delimiter: None,
            },
        )?;

        println!("{}  document:{}", YELLOW, RESET);
        println!("{}{}{}", YELLOW, document.to_string(), RESET);
        println!("{}{}{}", YELLOW, "-----------------------------------------------------------------------------------------------------", RESET);
    }

    println!();
    println!("{}=== End Basis Network Report ==={}", YELLOW, RESET);

    Ok(())
}

//pub async fn report_basis_clusters<P: Provider>(
//    provider: Arc<P>,
//    normalization_context: Arc<RwLock<NormalizationContext>>,
//) -> Result<(), Errors> {
//    let basis_clusters = {
//        let lock = read_lock!(normalization_context);
//        lock.basis_clusters
//            .as_ref()
//            .ok_or_else(|| {
//                Errors::DeficientNormalizationContextError("Basis clusters not provided in normalization context".to_string())
//            })?
//            .values()
//            .cloned()
//            .collect::<Vec<_>>()
//    };
//
//    let basis_networks = {
//        let lock = read_lock!(normalization_context);
//        lock.basis_networks
//            .as_ref()
//            .ok_or_else(|| {
//                Errors::DeficientNormalizationContextError("Basis networks not provided in normalization context".to_string())
//            })?
//            .clone()
//    };
//
//    println!("{}=== Basis Cluster Report ({} clusters) ==={}", MAGENTA, basis_clusters.len(), RESET);
//
//    for cluster in &basis_clusters {
//        println!("{}{}{}", MAGENTA, "-----------------------------------------------------------------------------------------------------", RESET);
//        println!("{}--- Cluster [{}] ---{}", MAGENTA, cluster.id.to_string(), RESET);
//        println!("{}  networks: {}{}", MAGENTA, cluster.networks.len(), RESET);
//        println!("{}  relationships: {}{}", MAGENTA, cluster.relationships.len(), RESET);
//        println!("{}  prompts: {:?}{}", MAGENTA, cluster.metadata.prompts, RESET);
//        println!("{}{}{}", MAGENTA, "-----------------------------------------------------------------------------------------------------", RESET);
//
//        for network_lineage in &cluster.networks {
//            let network = basis_networks.values().find(|n| &n.basis_lineages == network_lineage);
//            if let Some(network) = network {
//                println!("{}  Network [{}]{}", MAGENTA, network.id.to_string(), RESET);
//                println!("{}    basis_lineages: {}{}", MAGENTA, network.basis_lineages, RESET);
//                println!("{}    transformation: {}{}", MAGENTA, network.transformation.description, RESET);
//
//                let contexts = {
//                    let lock = read_lock!(normalization_context);
//                    lock.basis_network_contexts
//                        .as_ref()
//                        .ok_or_else(|| {
//                            Errors::DeficientNormalizationContextError("Basis network contexts not provided in normalization context".to_string())
//                        })?
//                        .get(&network.id)
//                        .map(|contexts| contexts.iter().take(3).cloned().collect::<Vec<_>>())
//                        .unwrap_or_default()
//                };
//
//                let graph_root: Graph = Arc::new(RwLock::new(GraphNode {
//                    id: ID::new(),
//                    parents: Vec::new(),
//                    description: String::new(),
//                    hash: Hash::new(),
//                    subgraph_hash: Hash::new(),
//                    lineage: Lineage::new(),
//                    children: Vec::new(),
//                }));
//
//                let normalized: Vec<NormalContext> = contexts
//                    .iter()
//                    .map(|context| {
//                        network.apply(
//                            Arc::clone(&normalization_context),
//                            context.clone(),
//                            Arc::clone(&graph_root),
//                        )
//                    })
//                    .collect::<Result<Vec<NormalContext>, Errors>>()?;
//
//                let mut normal_contexts: HashMap<ID, Arc<NormalContext>> = HashMap::new();
//                let mut contexts_lookup: HashMap<ID, Arc<NormalContext>> = HashMap::new();
//
//                for normal_context in normalized {
//                    let normal_context = Arc::new(normal_context);
//
//                    normal_contexts.insert(normal_context.id.clone(), Arc::clone(&normal_context));
//                    contexts_lookup.insert(
//                        read_lock!(normal_context.graph_node).id.clone(),
//                        Arc::clone(&normal_context)
//                    );
//                }
//
//                let root_normal_context = Arc::new(NormalContext {
//                    id: ID::new(),
//                    network_name: None,
//                    network_description: None,
//                    data_node: Arc::new(DataNode {
//                        id: ID::new(),
//                        hash: Hash::new(),
//                        lineage: Lineage::new(),
//                        fields: HashMap::new(),
//                        description: String::new(),
//                    }),
//                    graph_node: Arc::clone(&graph_root),
//                    contexts: Vec::new(),
//                });
//
//                normal_contexts.insert(root_normal_context.id.clone(), Arc::clone(&root_normal_context));
//                contexts_lookup.insert(
//                    read_lock!(root_normal_context.graph_node).id.clone(),
//                    Arc::clone(&root_normal_context)
//                );
//
//                let normal_meta_context = NormalMetaContext {
//                    contexts: normal_contexts,
//                    graph_root,
//                    contexts_lookup,
//                };
//
//                let document = Document::from_normal_meta_context(
//                    &normal_meta_context,
//                    &DocumentFormat {
//                        format_type: DocumentType::Json,
//                        encoding: Some(String::from("UTF-8")),
//                        indent: None,
//                        line_ending: None,
//                        headers: None,
//                        wrap_text: None,
//                        exclude_nulls: None,
//                        custom_delimiter: None,
//                    },
//                )?;
//
//                println!("{}    document:{}", MAGENTA, RESET);
//                println!("{}{}{}", MAGENTA, document.to_string(), RESET);
//            }
//        }
//
//        println!();
//        println!("{}  Relationships between networks:{}", MAGENTA, RESET);
//        for (rel_idx, relationship) in cluster.relationships.iter().enumerate() {
//            println!("{}    [{}] {:?}: {} -> {}{}", MAGENTA, rel_idx + 1, relationship.relationship, relationship.from, relationship.to, RESET);
//        }
//
//        println!("{}{}{}", MAGENTA, "-----------------------------------------------------------------------------------------------------", RESET);
//        println!();
//    }
//
//    println!("{}=== End Basis Cluster Report ==={}", MAGENTA, RESET);
//
//    Ok(())
//}
