use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::config::CONFIG;
use crate::context::Context;
use crate::data_node::DataNodeFields;
use crate::meta_context::MetaContext;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ContextGroup {
    pub acyclic_lineage: Lineage,
    pub lineage: Option<Lineage>,
    pub indexed_lineage: Option<Lineage>,
    pub fields: DataNodeFields,
    pub contexts: Vec<Arc<Context>>,
    pub snippets: Vec<String>,
}

static DEBUG_CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl ContextGroup {
    pub fn debug(&self) {
        let call = DEBUG_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("ContextGroup debug #{} — acyclic lineage: {}", call, self.acyclic_lineage.to_string());
        eprintln!("  contexts: {}", self.contexts.len());
        eprintln!("{}", "-".repeat(80));

        let mut by_lineage: Vec<(String, Vec<(usize, &Arc<Context>)>)> = {
            let mut map: HashMap<String, Vec<(usize, &Arc<Context>)>> = HashMap::new();
            for (i, context) in self.contexts.iter().enumerate() {
                map.entry(context.lineage.to_string())
                    .or_default()
                    .push((i, context));
            }
            let mut v: Vec<_> = map.into_iter().collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        };

        for (lineage_str, members) in &by_lineage {
            eprintln!("  lineage: {}", lineage_str);
            for (i, context) in members {
                let document_node = read_lock!(context.document_node);
                eprintln!("    [{}] element: {}  content: {}", i, document_node.get_element_name(), document_node.to_string());
            }
        }

        eprintln!("{}", "-".repeat(80));
        eprintln!("  snippets ({} shown):", self.snippets.len().min(2));
        for snippet in self.snippets.iter().take(2) {
            eprintln!("  ---");
            eprintln!("{}", snippet);
        }

        eprintln!("{}", "=".repeat(80));
    }

}
