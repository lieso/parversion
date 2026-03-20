use std::sync::{Arc, RwLock};

use crate::context_group::ContextGroup;
use crate::path::Path;
use crate::prelude::*;
use crate::schema_context::SchemaContext;
use crate::transformation::{FieldTransformation, SchemaTransformation, FieldMetadata, NetworkTransformation, NetworkMetadata};
use crate::context::Context;

mod openai;
mod translation;
mod categorization;
mod node_analysis;
mod network_analysis;

use node_analysis::NodeAnalysis;
use network_analysis::NetworkAnalysis;

pub struct LLM {}

impl LLM {
    pub async fn translate_schema_node(
        meta_context: Arc<RwLock<MetaContext>>,
        schema_context:SchemaContext,
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
            u64, // tokens
        ),
        Errors
    > {
        log::trace!("In categorize");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  CATEGORIZE GRAPH START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let (categorization_response, metadata) = categorization::Categorization::categorize_graph(
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
                .collect(),
            metadata.tokens
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

    pub async fn get_network_transformation(
        subgraph_hash: &str,
        json: &str,
        document_summary: &str
    ) -> Result<(
        NetworkTransformation,
        (
            u64 // tokens
        )
    ), Errors> {
        log::trace!("In get_network_transformation");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  NETWORK TRANSFORMATION START                 ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");

        let result = NetworkAnalysis::get_network_transformation(
            json,
            document_summary
        ).await?;

        let inference = result.data.unwrap();
        let meta = result.metadata;

        let network_transformation = NetworkTransformation {
            id: ID::new(),
            description: inference.description.clone(),
            subgraph_hash: subgraph_hash.to_string(),
            image: inference.name.to_string(),
            meta: NetworkMetadata {
                fields: inference.fields.clone(),
                cardinality: inference.cardinality.clone(),
                field_types: inference.field_types.clone(),
                context: inference.context.clone(),
                structure: inference.structure.clone(),
            }
        };

        Ok((network_transformation, (meta.tokens)))
    }

    pub async fn get_node_transformations(
        context_group: ContextGroup,
        document_summary: &str
    ) -> Result<(
        Vec<FieldTransformation>,
        (
            u64 // tokens
        )
    ), Errors> {
        log::trace!("In get_node_transformations");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  NODE TRANSFORMATION START                    ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");

        let mut field_transformations = Vec::new();
        let mut tokens: u64 = 0;

        for (field, value) in context_group.fields.into_iter() {
            let result = NodeAnalysis::get_node_transformation(
                &field,
                &value,
                context_group.snippets.clone(),
                document_summary
            ).await?;

            if let Some(field_inference_response) = result.data {
                let transformation = FieldTransformation {
                    id: ID::new(),
                    description: field_inference_response.description,
                    field: field.to_string(),
                    image: field_inference_response.field_name,
                    meta: FieldMetadata {
                        data_type: field_inference_response.data_type,
                    }
                };

                field_transformations.push(transformation);
            }

            tokens += result.metadata.tokens;
        }

        Ok((field_transformations, (tokens)))
    }

    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        openai::OpenAI::function_to_operation(&code).await
    }
}
