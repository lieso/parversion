use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};
use crate::graph_node::{GraphNode, GraphNodeID};
use crate::config::{CONFIG};
use crate::context_group::ContextGroup;

mod openai;

pub struct LLM {}

impl LLM {
    pub async fn get_field_transformations(
        meta_context: Arc<MetaContext>,
        context_group: ContextGroup,
    ) -> Result<Vec<FieldTransformation>, Errors> {
        log::trace!("In get_field_transformation");

        let example_snippet_count = read_lock!(CONFIG).llm.example_snippet_count;

        let mut field_transformations = Vec::new();

        for (field, value) in context_group.fields.into_iter() {
            match openai::OpenAI::get_field_transformation(
                &context_group.lineage,
                &field,
                &value,
                context_group.snippets.clone()
            ).await {
                Some(transformation) => field_transformations.push(transformation),
                None => {
                    log::info!("Field eliminated");
                }
            }
        }

        Ok(field_transformations)
    }

    pub async fn get_relationships(
        overall_context: String,
        target_subgraph_hash: String,
        subgraphs: Vec<(String, String)>
    ) -> Result<(), Errors> {
        log::trace!("In get_relationships");

        openai::OpenAI::get_relationships(
            overall_context.clone(),
            target_subgraph_hash.clone(),
            subgraphs.clone(),
        ).await;

        Ok(())
    }

    pub async fn get_summary(
        meta_context: &MetaContext,
    ) -> Result<String, Errors> {
        log::trace!("In get_summary");

        let compact_document = meta_context.get_original_document();

        let summary = openai::OpenAI::get_summary(
            compact_document.clone(),
        ).await?;

        Ok(summary)
    }
}
