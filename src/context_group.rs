use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::config::CONFIG;
use crate::context::Context;
use crate::data_node::DataNodeFields;
use crate::meta_context::MetaContext;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ContextGroup {
    pub lineage: Lineage,
    pub fields: DataNodeFields,
    pub contexts: Vec<Arc<Context>>,
    pub snippets: Vec<String>,
}

static DEBUG_CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl ContextGroup {
    pub fn debug(&self) {
        let call = DEBUG_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("ContextGroup debug #{} — acyclic lineage: {}", call, self.lineage.to_string());
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
                eprintln!("    [{}] indexed lineage: {}  element: {}  content: {}", i, context.indexed_lineage.to_string(), document_node.get_element_name(), document_node.to_string());
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

    #[deprecated]
    pub fn from_meta_context(meta_context: Arc<RwLock<MetaContext>>) -> Vec<Self> {
        log::trace!("In from_meta_context");

        let lock = read_lock!(meta_context);
        let contexts = lock.contexts.as_ref().unwrap();

        let mut context_groups: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
        let mut seen_context_ids: HashSet<ID> = HashSet::new();

        for context in contexts.values() {
            if seen_context_ids.insert(context.id.clone()) {
                context_groups
                    .entry(context.acyclic_lineage.clone())
                    .or_insert_with(Vec::new)
                    .push(context.clone());
            }
        }

        let example_snippet_count = read_lock!(CONFIG).llm.example_snippet_count;

        context_groups
            .into_iter()
            .map(|(lineage, contexts)| {
                let fields = contexts.first().unwrap().data_node.fields.clone();
                let snippets: Vec<String> = contexts
                    .iter()
                    .take(example_snippet_count)
                    .map(|context| context.generate_snippet(Arc::clone(&meta_context)))
                    .collect();

                ContextGroup {
                    lineage,
                    fields,
                    contexts,
                    snippets,
                }
            })
            .collect()
    }
}
