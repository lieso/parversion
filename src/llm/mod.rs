use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use rand::prelude::*;
use std::time::Duration;

use crate::basis_field::BasisField;
use crate::basis_network::BasisNetwork;
use crate::config::CONFIG;
use crate::prelude::*;
use crate::transformation::{
    FieldTransformation,
    FieldMetadata,
    NetworkTransformation,
    FieldTranslationTransformation,
    NetworkTranslationTransformation
};
use crate::context::Context;

mod document;
mod translation;

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
                &meta_context,
                Vec::new()
            )?
        };

        let target_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.target_meta_context.as_ref().unwrap();
            target_context.generate_context_string(
                &meta_context,
                Vec::new()
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
                &meta_context,
                Vec::new()
            )?
        };

        let target_context_string = {
            let lock = read_lock!(translation_context);
            let meta_context = lock.target_meta_context.as_ref().unwrap();
            target_context.generate_context_string(
                &meta_context,
                Vec::new()
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
