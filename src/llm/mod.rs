use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::context_group::ContextGroup;
use crate::path::Path;

mod openai;

pub struct LLM {}

impl LLM {
    pub async fn get_translation_schema(
        meta_context: Arc<RwLock<MetaContext>>,
        marked_schema: &String,
        target_schema: Arc<String>
    ) -> Result<Option<(
        String, // name
        String, // description
        String, // source path
        String // target path
    )>, Errors> {
        log::trace!("In get_translation_schema");

        let (maybe_json_path, maybe_source_path, maybe_target_path) = openai::OpenAI::match_schema_nodes(
            marked_schema,
            Arc::clone(&target_schema)
        ).await?;

        if let (Some(json_path), Some(source_path), Some(target_path)) = (maybe_json_path, maybe_source_path, maybe_target_path) {
            let translation_schema = {
                let lock = read_lock!(meta_context);
                lock.translation_schema.clone().unwrap()
            };

            let maybe_schema_node = translation_schema.get_schema_node_by_json_path(&json_path);

            if let Some(schema_node) = maybe_schema_node {
                return Ok(Some((
                    schema_node.name.clone(),
                    schema_node.description.clone(),
                    source_path,
                    target_path
                )));
            } else {
                log::warn!("Could not get schema node from target schema using LLM JSON path");
            }
        }

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
