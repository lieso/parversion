use std::sync::{Arc, RwLock};

use crate::normalization_context::NormalizationContext;
use crate::node_analysis::get_context_groups;
use crate::prelude::*;
use crate::provider::Provider;

const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
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
        println!("{}  prompt_hash: {:?}{}", CYAN, field.metadata.prompt_hash, RESET);
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
        println!("{}  prompt hash: {:?}{}", MAGENTA, group.metadata.prompt_hash, RESET);
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
