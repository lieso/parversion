use std::sync::{Arc, RwLock};

use crate::normalization_context::NormalizationContext;
use crate::node_analysis::get_context_groups;
use crate::prelude::*;
use crate::provider::Provider;

pub async fn report_basis_groups<P: Provider>(
    provider: Arc<P>,
    normalization_context: Arc<RwLock<NormalizationContext>>,
) -> Result<(), Errors> {
    let context_groups = {
        let lock = read_lock!(normalization_context);
        lock.context_groups
            .clone()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Context groups not provided in meta context".to_string())
            })?
    };
    let basis_groups = {
        let lock = read_lock!(normalization_context);
        lock.basis_groups
            .as_ref()
            .ok_or_else(|| {
                Errors::DeficientMetaContextError("Basis groups not provided in meta context".to_string())
            })?
            .values()
            .cloned()
            .collect::<Vec<_>>()
    };

    log::info!("=== Basis Group Report ({} groups) ===", basis_groups.len());

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

        log::info!("-----------------------------------------------------------------------------------------------------");
        log::info!("--- Group [{}] ---", lineage_desc);
        log::info!("  total contexts: {}", contexts.len());
        log::info!("-----------------------------------------------------------------------------------------------------");

        for (i, context) in contexts.iter().take(10).enumerate() {
            let fields: Vec<String> = context
                .data_node
                .fields
                .iter()
                .map(|(k, v)| format!("{}={:?}", k, v))
                .collect();
            log::info!("  [{}] {}", i + 1, fields.join(", "));
        }

        log::info!("\n");
    }

    log::info!("=== End Basis Group Report ===");

    Ok(())
}
