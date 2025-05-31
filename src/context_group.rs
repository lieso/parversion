use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};

use crate::prelude::*;
use crate::data_node::DataNodeFields;
use crate::config::{CONFIG};
use crate::meta_context::MetaContext;
use crate::context::Context;

#[derive(Clone, Debug)]
pub struct ContextGroup {
    pub lineage: Lineage,
    pub fields: DataNodeFields,
    pub contexts: Vec<Arc<Context>>,
    pub snippets: Vec<String>,
}

impl ContextGroup {
    pub fn from_meta_context(meta_context: Arc<RwLock<MetaContext>>) -> Vec<Self> {
        log::trace!("In from_meta_context");

        let lock = read_lock!(meta_context);
        let contexts = lock.contexts.ok_or(Errors::ContextsNotProvided)?;

        let mut context_groups: HashMap<Lineage, Vec<Arc<Context>>> = HashMap::new();
        let mut seen_context_ids: HashSet<ID> = HashSet::new();

        for context in meta_context.contexts.values() {
            if seen_context_ids.insert(context.id.clone()) {
                context_groups
                    .entry(context.lineage.clone())
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
                    snippets
                }
            })
            .collect()
    }
}
