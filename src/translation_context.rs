use std::collections::HashMap;
use std::sync::Arc;

use crate::prelude::*;
use crate::context::Context;
use crate::translation_node::TranslationNode;

pub struct TranslationContext {
    pub input_meta_context: Option<Arc<MetaContext>>,
    pub target_meta_context: Option<Arc<MetaContext>>,
    pub translation_nodes: Option<HashMap<ID, Arc<TranslationNode>>>,
}

impl TranslationContext {
    pub fn new() -> Self {
        Self {
            input_meta_context: None,
            target_meta_context: None,
            translation_nodes: None,
        }
    }

    pub fn must_get_unique_input_contexts(&self) -> Result<Vec<Arc<Context>>, Errors> {
        Self::unique_contexts_from(&self.input_meta_context)
    }

    pub fn must_get_unique_target_contexts(&self) -> Result<Vec<Arc<Context>>, Errors> {
        Self::unique_contexts_from(&self.target_meta_context)
    }

    pub fn update_meta_contexts(
        &mut self,
        input_meta_context: MetaContext,
        target_meta_context: MetaContext,
    ) {
        self.input_meta_context = Some(Arc::new(input_meta_context));
        self.target_meta_context = Some(Arc::new(target_meta_context));
    }

    pub fn update_translation_nodes(&mut self, nodes: HashMap<ID, Arc<TranslationNode>>) {
        self.translation_nodes = Some(nodes);
    }

    fn unique_contexts_from(maybe_meta_context: &Option<Arc<MetaContext>>) -> Result<Vec<Arc<Context>>, Errors> {
        let meta_context = maybe_meta_context.as_ref().ok_or_else(|| {
            Errors::DeficientTranslationContextError("Meta context missing in translation context".to_string())
        })?;

        let contexts = meta_context.contexts.values()
            .filter(|c| !c.data_node.fields.is_empty())
            .cloned()
            .collect();

        Ok(contexts)
    }
}
