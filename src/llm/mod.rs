use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use rand::prelude::*;
use std::time::Duration;

use crate::basis_field::BasisField;
use crate::basis_network::BasisNetwork;
use crate::config::CONFIG;
use crate::network_relationship::NetworkRelationshipType;
use crate::prelude::*;
use crate::transformation::{
    FieldTransformation,
    FieldMetadata,
    NetworkTransformation,
    NetworkMetadata,
    FieldTranslationTransformation,
    NetworkTranslationTransformation
};
use crate::context::Context;

mod network_analysis;
mod network_relationships;
mod document;
mod translation;

use network_analysis::NetworkAnalysis;
use network_relationships::NetworkRelationships;
use document::Document;
use translation::Translation;

pub struct LLM {}

impl LLM {
    pub async fn schema_to_instance(
        schema: String
    ) -> Result<(String, (u64,)), Errors> {
        log::trace!("In schema_to_instance");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  SCHEMA TO INSTANCE START                     ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (response, metadata) = Document::schema_to_instance(schema).await?;

        Ok((response.instance_document, (metadata.tokens,)))
    }

    pub async fn get_parent_child_link(
        snippet: String,
    ) -> Result<((String, Vec<(String, String)>, Vec<(String, String)>, String), (u64,)), Errors> {
        log::trace!("In get_parent_child_link");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  PARENT CHILD LINK START                      ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (response, metadata) = NetworkRelationships::get_parent_child_link(&snippet).await?;

        log::debug!("response: {:?}", response);

        let parent_value_xpaths = response.parent_value_xpaths
            .into_iter()
            .map(|v| (v.name, v.xpath))
            .collect();

        let candidate_value_xpaths = response.candidate_value_xpaths
            .into_iter()
            .map(|v| (v.name, v.xpath))
            .collect();

        Ok((
            (response.candidate_xpath, parent_value_xpaths, candidate_value_xpaths, response.filter_function),
            (metadata.tokens,),
        ))
    }

    pub async fn get_composition_link(
        snippet: String,
    ) -> Result<((String, String, String), (u64,)), Errors> {
        log::trace!("In get_composition_link");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  COMPOSITION LINK START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (response, metadata) = NetworkRelationships::get_composition_link(&snippet).await?;

        Ok(((response.forward_xpath, response.reverse_xpath, response.merged_variable_name), (metadata.tokens,)))
    }

    pub async fn identify_relationships(
        _meta_context: Arc<RwLock<NormalizationContext>>,
        original_document: String,
        network_jsons: Vec<(Arc<BasisNetwork>, Vec<String>)>
    ) -> Result<(Vec<(Arc<BasisNetwork>, Arc<BasisNetwork>, NetworkRelationshipType)>, (u64,)), Errors> {

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  IDENTIFY RELATIONSHIPS START                 ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let all_network_jsons: String = network_jsons.iter()
            .map(|(network, json_examples)| {
                let examples_string: String = json_examples.iter().enumerate()
                    .map(|(index, json)| format!("\nExample {}:\n{}\n", index + 1, json))
                    .collect();
                format!(
                    "\n{}\n\n[Network ID]\n{}\n\n[Network examples]\n{}\n",
                    "=".repeat(100),
                    network.id.to_string(),
                    examples_string
                )
            })
            .collect();

        let (relationships_response, metadata) = NetworkRelationships::identify_relationships(
            &original_document,
            &all_network_jsons,
        ).await?;

        log::debug!("relationships: {:?}", relationships_response.relationships);

        let relationships = relationships_response.relationships
            .into_iter()
            .map(|item| {
                let from = network_jsons.iter()
                    .find(|(n, _)| n.id.to_string() == item.from)
                    .map(|(n, _)| Arc::clone(n))
                    .unwrap_or_else(|| panic!("Relationship 'from' network not found: {}", item.from));
                let to = network_jsons.iter()
                    .find(|(n, _)| n.id.to_string() == item.to)
                    .map(|(n, _)| Arc::clone(n))
                    .unwrap_or_else(|| panic!("Relationship 'to' network not found: {}", item.to));
                let rel_type = serde_json::from_value(serde_json::Value::String(item.relationship_type.clone()))
                    .unwrap_or_else(|_| panic!("Unknown relationship type: {}", item.relationship_type));
                (from, to, rel_type)
            })
            .collect();

        Ok((relationships, (metadata.tokens,)))
    }

    pub async fn check_redundancy(
        _meta_context: Arc<RwLock<NormalizationContext>>,
        original_document: String,
        all_network_jsons: String
    ) -> Result<(Vec<String>, (u64,)), Errors> {

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  CHECK REDUNDANCY START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (redundancy_response, metadata) = NetworkRelationships::get_canonical_networks(
            &original_document,
            &all_network_jsons,
        ).await?;

        log::debug!("eliminated networks: {:?}", redundancy_response.eliminated);

        Ok((redundancy_response.canonical, (metadata.tokens,)))
    }

    pub async fn get_network_transformation(
        subgraph_hash: &str,
        json_examples: &[String],
        document_summary: &str
    ) -> Result<(
        NetworkTransformation,
        u64 
    ), Errors> {
        log::trace!("In get_network_transformation");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  NETWORK TRANSFORMATION START                 ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = NetworkAnalysis::get_network_transformation(
            json_examples,
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

    pub async fn get_node_translation(
        translation_context: Arc<RwLock<TranslationContext>>,
        input_context: Arc<Context>,
        target_context: Arc<Context>
    ) -> Result<(
        Vec<FieldTranslationTransformation>,
        (u64,)
    ), Errors> {
        log::trace!("In get_node_translation");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let input_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.input_meta_context.as_ref().unwrap();
            input_context.generate_context_string(
                &meta_context
            )?
        };

        let target_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.target_meta_context.as_ref().unwrap();
            target_context.generate_context_string(
                &meta_context
            )?
        };

        let user_prompt = format!(r##"
            [FIRST DOCUMENT]
            {}
            
            [SECOND DOCUMENT]
            {}
        "##, input_context_string, target_context_string);

        let (response, metadata) = Translation::translate_nodes(
            &user_prompt
        ).await?;

        let transformations: Vec<FieldTranslationTransformation> = response
            .matches
            .iter()
            .map(|node_match| {
                FieldTranslationTransformation {
                    id: ID::new(),
                    field: node_match.source_key.clone(),
                    image: node_match.target_key.clone(),
                    code: node_match.transform_code.clone()
                }
            })
            .collect();

        Ok((transformations, (metadata.tokens,)))
    }
    
    pub async fn get_network_translation(
        translation_context: Arc<RwLock<TranslationContext>>,
        input_context: Arc<Context>,
        target_context: Arc<Context>,
    ) -> Result<(
        Option<NetworkTranslationTransformation>,
        (u64,)
    ), Errors> {
        log::trace!("In get_network_translation");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let input_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.input_meta_context.as_ref().unwrap();
            input_context.generate_context_string(
                &meta_context
            )?
        };

        let target_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.target_meta_context.as_ref().unwrap();
            target_context.generate_context_string(
                &meta_context
            )?
        };

        let user_prompt = format!(r##"
            [FIRST DOCUMENT]
            {}
            
            [SECOND DOCUMENT]
            {}
        "##, input_context_string, target_context_string);

        let (response, metadata) = Translation::translate_networks(
            &user_prompt
        ).await?;

        let transformation = if response.is_match {
            Some(NetworkTranslationTransformation {
                id: ID::new(),
                image: target_context.network_name.clone(),
                cardinality: response.target_cardinality.clone(),
            })
        } else {
            None
        };

        Ok((transformation, (metadata.tokens,)))
    }
}
