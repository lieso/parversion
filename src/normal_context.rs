use std::sync::{Arc, RwLock};

use crate::context::Context;
use crate::data_node::DataNode;
use crate::graph_node::GraphNode;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct NormalContext {
    pub id: ID,
    pub network_name: Option<String>,
    pub network_description: Option<String>,
    pub graph_node: Arc<RwLock<GraphNode>>,
    pub data_node: Arc<DataNode>,
    pub contexts: Vec<Arc<Context>>,
}

impl NormalContext {
    pub fn generate_context_string_translation_node(
        &self,
        translation_context: Arc<RwLock<TranslationContext>>,
        field: &String,
    ) -> Result<String, Errors> {
        let groups = Self::group_contexts_by_normalization_context(
            self.contexts.clone(),
            read_lock!(translation_context).source.clone(),
        )?;

        let source_document = groups.iter().try_fold(
            String::new(),
            |acc, (normalization_context, contexts)| -> Result<String, Errors> {
                let context_string = Context::generate_spatial_context_multi(
                    Arc::clone(&normalization_context),
                    contexts.clone(),
                )?;

                if acc.is_empty() {
                    Ok(context_string)
                } else {
                    Ok(format!("{}\n\n{}", acc, context_string))
                }
            },
        )?;

        let object: String = serde_json::to_string_pretty(
            &self
                .data_node
                .fields
                .iter()
                .map(|(key, value)| (key.clone(), serde_json::Value::String(value.clone())))
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        )?;

        let result = format!(
            r##"
[FIELD]
{}

[OBJECT]
{}

[SOURCE DOCUMENT]
{}
        "##,
            field, object, source_document
        );

        Ok(result)
    }

    fn group_contexts_by_normalization_context(
        contexts: Vec<Arc<Context>>,
        normalization_contexts: Vec<Arc<RwLock<NormalizationContext>>>,
    ) -> Result<Vec<(Arc<RwLock<NormalizationContext>>, Vec<Arc<Context>>)>, Errors> {
        let mut groups = Vec::with_capacity(normalization_contexts.len());

        for normalization_context in normalization_contexts {
            let contexts_lookup = {
                let lock = read_lock!(normalization_context);
                let contexts_lookup = &lock
                    .meta_context
                    .as_ref()
                    .ok_or_else(|| {
                        Errors::DeficientNormalizationContextError(
                            "Meta context not provided in normalization context".to_string(),
                        )
                    })?
                    .contexts_lookup;

                Arc::clone(&contexts_lookup)
            };

            let members: Vec<Arc<Context>> = contexts
                .iter()
                .filter(|context| contexts_lookup.contains_key(&context.id))
                .cloned()
                .collect();

            groups.push((Arc::clone(&normalization_context), members));
        }

        Ok(groups)
    }
}
