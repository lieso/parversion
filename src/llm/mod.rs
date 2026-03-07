use std::sync::{Arc, RwLock};

use crate::context_group::ContextGroup;
use crate::path::Path;
use crate::prelude::*;
use crate::schema_context::SchemaContext;
use crate::transformation::{FieldTransformation, SchemaTransformation};

mod openai;
mod translation;
mod categorization;

pub struct LLM {}

impl LLM {
    pub async fn translate_schema_node(
        meta_context: Arc<RwLock<MetaContext>>,
        schema_context: SchemaContext,
        target_schema: Arc<String>,
    ) -> Result<Option<(Path, Path)>, Errors> {
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

        let match_response =
            translation::Translation::match_target_schema(&snippet, &target_schema).await?;

        log::debug!("match_response: {:?}", match_response);

        if match_response.is_incompatible || match_response.json_path.is_none() {
            log::debug!("Schema node is incompatible with target schema");
            return Ok(None);
        }

        let json_path = match_response.json_path.unwrap();

        let translation_schema = {
            let lock = read_lock!(meta_context);
            lock.translation_schema.clone().unwrap()
        };

        let maybe_schema_node = translation_schema.get_schema_node_by_json_path(&json_path);

        if let Some(target_schema_node) = maybe_schema_node {
            log::info!("Found target schema node");

            let (schema_node_path, target_node_path) = {
                let lock = read_lock!(meta_context);

                let schema_contexts = lock.schema_contexts.clone().unwrap();
                let schema_node_path: Path = schema_context.to_path(schema_contexts)?;

                let target_schema_context = lock
                    .translation_schema_contexts
                    .as_ref()
                    .unwrap()
                    .values()
                    .find(|sc| sc.schema_node.id == target_schema_node.id)
                    .unwrap();

                let target_schema_contexts = lock.translation_schema_contexts.clone().unwrap();
                let target_node_path: Path =
                    target_schema_context.to_path(target_schema_contexts)?;
                let target_node_path = target_node_path.with_unique_variables(&schema_node_path);

                (schema_node_path, target_node_path)
            };

            let variable_mapping = translation::Translation::match_path_variables(
                &schema_node_path,
                &target_node_path,
                &snippet,
                &target_schema,
            )
            .await?;

            log::debug!("variable_mapping: {:?}", variable_mapping);

            let target_node_path = target_node_path.with_mapped_variables(&variable_mapping);

            log::debug!("schema_node_path: {}", schema_node_path.to_string());
            log::debug!("target_node_path: {}", target_node_path.to_string());

            return Ok(Some((schema_node_path, target_node_path)));
        } else {
            log::warn!(
                "Schema node determined to be compatible but could not find target schema node"
            );
        }

        Ok(None)
    }

    pub async fn categorize(document: String) -> Result<
        (
            String, // name
            String, // description
            String, // structure
            Vec<String>, // aliases
        ),
        Errors
    > {
        log::trace!("In categorize");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  CATEGORIZE GRAPH START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let categorization_response = categorization::Categorization::categorize_graph(
            &document
        ).await?;

        let result = (
            categorization_response.category,
            categorization_response.description,
            categorization_response.structure,
            categorization_response.one_word_aliases
                .iter()
                .chain(
                    &categorization_response.two_word_aliases
                )
                .cloned()
                .collect()
        );

        Ok(result)
    }

    pub async fn get_normal_schema(
        marked_schema: &String,
    ) -> Result<
        (
            String,      // key
            String,      // description
            Vec<String>, // aliases
            Path,
        ),
        Errors,
    > {
        log::trace!("In get_normal_schema");

        unimplemented!()
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
                context_group.snippets.clone(),
            )
            .await
            {
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
        subgraphs: Vec<(String, String)>,
    ) -> Result<(String, Vec<String>, String), Errors> {
        log::trace!("In get_relationships");

        let (name, matches, description) = openai::OpenAI::get_relationships(
            overall_context.clone(),
            target_subgraph_hash.clone(),
            subgraphs.clone(),
        )
        .await?;

        Ok((name, matches, description))
    }

    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        openai::OpenAI::function_to_operation(&code).await
    }
}
