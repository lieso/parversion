use std::sync::{Arc, RwLock};

use crate::group_analysis::resolve_context_groups;
use crate::normalization_context::NormalizationContext;
use crate::prelude::*;
use crate::provider::Provider;

const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const GREEN: &str = "\x1b[32m";
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

            let mut current_data_node = Arc::clone(&context.data_node);

            for (txn_idx, transformation) in basis_node.transformations.iter().enumerate() {
                println!("{}    [Transformation {}] {}{}", GREEN, txn_idx + 1, transformation.description, RESET);
                println!("{}      field: {}, image: {}{}", GREEN, transformation.field, transformation.image, RESET);

                match transformation.transform(Arc::clone(&current_data_node)) {
                    Ok(transformed) => {
                        current_data_node = Arc::new(transformed);
                        println!("{}      After: {:?}{}", GREEN, current_data_node.fields, RESET);
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
