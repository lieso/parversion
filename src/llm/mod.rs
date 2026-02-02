use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::transformation::{FieldTransformation, SchemaTransformation};
use crate::context_group::ContextGroup;
use crate::path::Path;
use crate::schema_context::SchemaContext;

mod openai;
mod translation;

pub struct LLM {}

impl LLM {
    pub async fn translate_schema_node(
        meta_context: Arc<RwLock<MetaContext>>,
        schema_context: SchemaContext,
        target_schema: Arc<String>
    ) -> Result<Option<SchemaTransformation>, Errors> {
        log::trace!("In translate_schema_node");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  TRANSLATE SCHEMA NODE START                  ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");
        log::debug!("┌─── SCHEMA NODE ───────────────────────────────────────────────┐");
        log::debug!("{}", schema_context.schema_node.to_string());
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");

        let snippet = schema_context.generate_snippet(Arc::clone(&meta_context));

        let match_response = translation::Translation::match_target_schema(
            &snippet,
            &target_schema
        ).await?;

        log::debug!("match_response: {:?}", match_response);

        log::debug!("");
        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  TRANSLATE SCHEMA NODE END                    ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        Ok(None)
    }

    pub async fn get_normal_schema(marked_schema: &String) -> Result<(
        String, // key
        String, // description
        Vec<String>, // aliases
        Path
    ), Errors> {
        log::trace!("In get_normal_schema");

        unimplemented!()
    }

    pub async fn categorize_and_summarize(document: String) -> Result<(
        String, // name
        String, // description
        String // structure
    ), Errors> {
        log::trace!("In categorize_and_summarize");

        let (name, description, structure) = openai::OpenAI::categorize_summarize(&document).await?;

        Ok((name, description, structure))
    }

    pub async fn get_field_transformations(
        context_group: ContextGroup,
    ) -> Result<Vec<FieldTransformation>, Errors> {
        log::trace!("In get_field_transformation");

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
    ) -> Result<(String, Vec<String>, String), Errors> {
        log::trace!("In get_relationships");

        let (name, matches, description) = openai::OpenAI::get_relationships(
            overall_context.clone(),
            target_subgraph_hash.clone(),
            subgraphs.clone(),
        ).await?;

        Ok((name, matches, description))
    }

    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        openai::OpenAI::function_to_operation(&code).await
    }
}
