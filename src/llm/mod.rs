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
        target_json: String,
        other_subgraphs: Vec<String>
    ) -> Result<(), Errors> {
        log::trace!("In get_relationships");

        openai::OpenAI::get_relationships(
            target_json.clone(),
            other_subgraphs.clone(),
        ).await;

        Ok(())
    }
}
